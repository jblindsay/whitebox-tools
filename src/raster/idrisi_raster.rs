use std::io::Error;
use std::io::ErrorKind;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;
use std::f64;
use std::fs::File;
use std::mem;
use raster::*;
use io_utils::Endianness;

pub fn read_idrisi(file_name: &String,
                   configs: &mut RasterConfigs,
                   data: &mut Vec<f64>)
                   -> Result<(), Error> {
    // read the header file
    let header_file = file_name.replace(".rst", ".rdc");
    let f = try!(File::open(header_file));
    let f = BufReader::new(f);

    for line in f.lines() {
        let line_unwrapped = line.unwrap();
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        let line_split = line_unwrapped.split(":");
        let vec = line_split.collect::<Vec<&str>>();
        if vec[0].to_lowercase().contains("min. value") &&
           !vec[0].to_lowercase().contains("lineage") {
            configs.minimum = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("max. value") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.maximum = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("display min") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.display_min = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("display max") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.display_max = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("max. y") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.north = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("min. y") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.south = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("max. x") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.east = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("min. x") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.west = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("columns") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.columns = vec[1].trim().to_string().parse::<usize>().unwrap();
        } else if vec[0].to_lowercase().contains("rows") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.rows = vec[1].trim().to_string().parse::<usize>().unwrap();
        } else if vec[0].to_lowercase().contains("data type") &&
                  !vec[0].to_lowercase().contains("lineage") {
            if vec[1]
                   .trim()
                   .to_lowercase()
                   .to_string()
                   .contains("real") {
                configs.data_type = DataType::F32;
            } else if vec[1]
                          .trim()
                          .to_lowercase()
                          .to_string()
                          .contains("int") {
                configs.data_type = DataType::I16;
            } else if vec[1]
                          .trim()
                          .to_lowercase()
                          .to_string()
                          .contains("byte") {
                configs.data_type = DataType::U8;
            } else if vec[1]
                          .trim()
                          .to_lowercase()
                          .to_string()
                          .contains("rgb24") {
                configs.data_type = DataType::RGB24; //U32;
                configs.photometric_interp = PhotometricInterpretation::RGB; //Rgb24;
            }
        } else if vec[0].to_lowercase().contains("value units") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.z_units = vec[1].trim().to_string();
        } else if vec[0].to_lowercase().contains("ref.") &&
                  vec[0].to_lowercase().contains("units") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.xy_units = vec[1].trim().to_string();
        } else if vec[0].to_lowercase().contains("ref.") &&
                  vec[0].to_lowercase().contains("system") &&
                  !vec[0].to_lowercase().contains("lineage") {
            configs.coordinate_ref_system_wkt = vec[1].trim().to_string();
        } else if vec[0].to_lowercase().contains("byteorder") &&
                  !vec[0].to_lowercase().contains("lineage") {
            if vec[1].trim().to_lowercase().contains("little_endian") ||
               vec[1].trim().to_lowercase().contains("lsb") {
                configs.endian = Endianness::LittleEndian;
            } else {
                configs.endian = Endianness::BigEndian;
            }
        } else if vec[0].to_lowercase().contains("lineage") ||
                  vec[0].to_lowercase().contains("comment") {
            configs.metadata.push(vec[1].trim().to_string());
        } else if vec[0].to_lowercase().contains("file type") &&
                  !vec[0].to_lowercase().contains("lineage") {
            if !vec[1].trim().to_lowercase().contains("binary") ||
               vec[1].trim().to_lowercase().contains("packed") {
                return Err(Error::new(ErrorKind::InvalidInput,
                                      "Idrisi ASCII and packed binary files are currently unsupported."));
            }
        }
    }

    configs.resolution_x = (configs.east - configs.west) / configs.columns as f64;
    configs.resolution_y = (configs.north - configs.south) / configs.rows as f64;

    // read the data file
    let data_file = file_name.replace(".rdc", ".rst");
    let mut f = try!(File::open(data_file.clone()));

    let data_size = if configs.data_type == DataType::F32 {
        4
    } else if configs.data_type == DataType::U32 {
        3 // rgb is actually 3 bytes
    } else if configs.data_type == DataType::I16 {
        2
    } else {
        // DataType::Byte
        1
    };

    let num_cells = configs.rows * configs.columns;
    let buf_size = 1_000_000usize;
    let mut j = 0;
    while j < num_cells {
        let mut buffer = vec![0; buf_size * data_size];

        try!(f.read(&mut buffer));

        // read the file's bytes into a buffer
        //try!(f.read_to_end(&mut buffer));

        //try!(br.fill_buf().unwrap()(&mut buffer));

        let mut offset: usize;
        match configs.data_type {
            DataType::F32 => {
                for i in 0..buf_size {
                    offset = i * data_size;
                    data.push(unsafe {
                                  mem::transmute::<[u8; 4], f32>([buffer[offset],
                                                                  buffer[offset + 1],
                                                                  buffer[offset + 2],
                                                                  buffer[offset + 3]])
                              } as f64);
                    j += 1;
                    if j == num_cells {
                        break;
                    }
                }
            }
            DataType::U32 => {
                //RGB
                for i in 0..buf_size {
                    offset = i * data_size;
                    data.push(unsafe {
                                  mem::transmute::<[u8; 4], u32>([buffer[offset],
                                                                  buffer[offset + 1],
                                                                  buffer[offset + 2],
                                                                  255])
                              } as f64);
                    j += 1;
                    if j == num_cells {
                        break;
                    }
                }
            }
            DataType::I16 => {
                for i in 0..buf_size {
                    offset = i * data_size;
                    data.push(unsafe {
                                  mem::transmute::<[u8; 2], i16>([buffer[offset],
                                                                  buffer[offset + 1]])
                              } as f64);
                    j += 1;
                    if j == num_cells {
                        break;
                    }
                }
            }
            DataType::U8 => {
                for i in 0..buf_size {
                    data.push(buffer[i] as f64);
                    j += 1;
                    if j == num_cells {
                        break;
                    }
                }
            }
            _ => {
                return Err(Error::new(ErrorKind::NotFound, "Raster data type is unknown."));
            }
        }
    }

    Ok(())
}

pub fn write_idrisi<'a>(r: &'a mut Raster) -> Result<(), Error> {
    // figure out the minimum and maximum values
    for val in &r.data {
        let v = *val;
        if v != r.configs.nodata {
            if v < r.configs.minimum {
                r.configs.minimum = v;
            }
            if v > r.configs.maximum {
                r.configs.maximum = v;
            }
        }
    }

    if r.configs.display_min == f64::INFINITY {
        r.configs.display_min = r.configs.minimum;
    }
    if r.configs.display_max == f64::NEG_INFINITY {
        r.configs.display_max = r.configs.maximum;
    }

    // Save the header file
    let header_file = r.file_name.replace(".rst", ".rdc");
    let f = try!(File::create(header_file));
    let mut writer = BufWriter::new(f);

    try!(writer.write_all("file format : IDRISI Raster A.1\n".as_bytes()));

    try!(writer.write_all(format!("file title  : {}\n", r.configs.title).as_bytes()));

    match r.configs.data_type {
        DataType::F32 => {
            try!(writer.write_all("data type   : real\n".as_bytes()));
        }
        DataType::U32 => {
            // rgb
            try!(writer.write_all("data type   : RGB24\n".as_bytes()));
        }
        DataType::I16 => {
            try!(writer.write_all("data type   : integer\n".as_bytes()));
        }
        DataType::U8 => {
            try!(writer.write_all("data type   : byte\n".as_bytes()));
        }
        _ => {
            return Err(Error::new(ErrorKind::NotFound,
                                  format!("Raster data type {:?} not supported in this format.",
                                          r.configs.data_type)));
        }
    }

    try!(writer.write_all("file type   : binary\n".as_bytes()));

    let s = format!("columns     : {}\n", r.configs.columns);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("rows        : {}\n", r.configs.rows);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("ref. system : {}\n", r.configs.coordinate_ref_system_wkt);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("ref. units  : {}\n", r.configs.xy_units);
    try!(writer.write_all(s.as_bytes()));

    try!(writer.write_all("unit dist.  : 1.0000000\n".as_bytes()));

    let s = format!("min. X      : {}\n", r.configs.west);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("max. X      : {}\n", r.configs.east);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("min. Y      : {}\n", r.configs.south);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("max. Y      : {}\n", r.configs.north);
    try!(writer.write_all(s.as_bytes()));

    try!(writer.write_all("pos'n error : unknown\n".as_bytes()));

    try!(writer.write_all("resolution  : unknown\n".as_bytes()));

    let s = format!("min. value  : {}\n", r.configs.minimum);
    try!(writer.write_all(s.as_bytes())); //.expect("Unable to write data)

    let s = format!("max. value  : {}\n", r.configs.maximum);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("display min : {}\n", r.configs.display_min);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("display max : {}\n", r.configs.display_max);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("value units : {}\n", r.configs.z_units);
    try!(writer.write_all(s.as_bytes()));

    try!(writer.write_all("value error : unknown\n".as_bytes()));

    try!(writer.write_all("flag value  : none\n".as_bytes()));

    try!(writer.write_all("flag def'n  : none\n".as_bytes()));

    try!(writer.write_all("legend cats : 0\n".as_bytes()));

    try!(writer.write_all("byteorder   : LITTLE_ENDIAN\n".as_bytes()));

    for md in &r.configs.metadata {
        let s = format!("comment     : {}\n", md.replace(":", ";"));
        try!(writer.write_all(s.as_bytes()));
    }

    let _ = writer.flush();


    // read the data file
    let data_file = r.file_name.replace(".rdc", ".rst");
    let f = try!(File::create(&data_file));
    let mut writer = BufWriter::new(f);

    let mut u16_bytes: [u8; 2];
    //let mut u24_bytes: [u8; 3];
    let mut u32_bytes: [u8; 4];

    let num_cells: usize = r.configs.rows * r.configs.columns;
    match r.configs.data_type {
        DataType::F32 => {
            for i in 0..num_cells {
                u32_bytes = unsafe { mem::transmute(r.data[i] as f32) };
                try!(writer.write(&u32_bytes));
            }
        }
        DataType::U32 => {
            // rgb data
            return Err(Error::new(ErrorKind::Other,
                                  "Writing RGB24 raster is not currently supported."));
            // for i in 0..num_cells {
            //     u24_bytes = unsafe { mem::transmute(r.data[i] as u32) };
            //     try!(writer.write(&u16_bytes));
            // }
        }
        DataType::I16 => {
            for i in 0..num_cells {
                u16_bytes = unsafe { mem::transmute(r.data[i] as u16) };
                try!(writer.write(&u16_bytes));
            }
        }
        DataType::U8 => {
            for i in 0..num_cells {
                try!(writer.write(&[r.data[i] as u8]));
            }
        }
        _ => {
            return Err(Error::new(ErrorKind::NotFound, "Raster data type is unknown."));
        }
    }

    let _ = writer.flush();

    Ok(())
}
