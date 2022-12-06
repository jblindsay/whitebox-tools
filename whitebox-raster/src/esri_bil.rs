use super::*;
use whitebox_common::utils::Endianness;
use std::convert::TryInto;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Error, SeekFrom};

pub fn read_esri_bil(
    file_name: &String,
    configs: &mut RasterConfigs,
    data: &mut Vec<f64>,
) -> Result<(), Error> {
    // read the header file
    let header_file = Path::new(&file_name)
        .with_extension("hdr")
        .into_os_string()
        .into_string()
        .expect("Error creating header file name string for BIL file.");
    let f = File::open(header_file).expect("Error opening BIL header (HDR) file.");
    let f = BufReader::new(f);

    // let mut nbands = 1usize;
    let mut band_row_bytes = 0usize;
    let mut total_row_bytes = 0u64;
    let mut pixel_type = String::new();
    let mut nbits = 0usize;
    let mut ulxmap = 0f64;
    let mut ulymap = 0f64;
    configs.nodata = -32768f64; // default in event that it is not in header file

    for line in f.lines() {
        let line_unwrapped = line.unwrap();
        let line_split = line_unwrapped.split(" ");
        let vec = line_split.collect::<Vec<&str>>();
        let key = vec[0].to_lowercase().trim().to_string();
        let mut value = String::new();
        for i in 1..vec.len() {
            let v = vec[i];
            if !v.trim().is_empty() {
                value = v.to_lowercase().to_string();
                break;
            }
        }

        if key.contains("byteorder") {
            if value.contains("i") {
                configs.endian = Endianness::LittleEndian;
            } else {
                configs.endian = Endianness::BigEndian;
            }
        } else if key.contains("layout") {
            if !value.contains("bil") {
                println!("Warning: Only the Esri BIL layout is supported by WhiteboxTools. BSQ and BIP layouts are currently unsupported.")
            }
        } else if key.contains("nrows") {
            configs.rows = value.trim().parse::<f32>().unwrap() as usize;
        } else if key.contains("ncols") {
            configs.columns = value.trim().parse::<f32>().unwrap() as usize;
        } else if key.contains("nbands") {
            let nbands = value.trim().parse::<f32>().unwrap() as usize;
            if nbands > 1 {
                if !value.contains("bil") {
                    println!("Warning: The Esri BIL reader only supports single-band rasters. Only the first band will be read.")
                }
            }
        } else if key.contains("nbits") {
            nbits = value.trim().parse::<f32>().unwrap() as usize;
        } else if key.contains("bandrowbytes") {
            band_row_bytes = value.trim().parse::<f64>().unwrap() as usize;
        } else if key.contains("totalrowbytes") {
            total_row_bytes = value.trim().parse::<f64>().unwrap() as u64;
        } else if key.contains("pixeltype") {
            pixel_type = value;
        } else if key.contains("ulxmap") {
            ulxmap = value.trim().parse::<f64>().unwrap();
        } else if key.contains("ulymap") {
            ulymap = value.trim().parse::<f64>().unwrap();
        } else if key.contains("xdim") {
            configs.resolution_x = value.trim().parse::<f64>().unwrap();
        } else if key.contains("ydim") {
            configs.resolution_y = value.trim().parse::<f64>().unwrap();
        } else if key.contains("nodata") {
            configs.nodata = value.trim().parse::<f64>().unwrap();
        }
    }

    configs.photometric_interp = PhotometricInterpretation::Continuous;

    if pixel_type == "unsignedint" {
        match nbits {
            8 => configs.data_type = DataType::U8,
            16 => configs.data_type = DataType::U16,
            32 => configs.data_type = DataType::U32,
            _ => panic!("Unrecognized data type"),
        }
    } else if pixel_type == "signedint" {
        match nbits {
            8 => configs.data_type = DataType::I8,
            16 => configs.data_type = DataType::I16,
            32 => configs.data_type = DataType::I32,
            _ => panic!("Unrecognized data type"),
        }
    } else {
        // float
        match nbits {
            32 => configs.data_type = DataType::F32,
            64 => configs.data_type = DataType::F64,
            _ => panic!("Unrecognized data type"),
        }
    }

    configs.north = ulymap + configs.resolution_y / 2.0f64;
    configs.west = ulxmap - configs.resolution_x / 2.0f64;
    configs.south = configs.north - configs.resolution_y * configs.rows as f64;
    configs.east = configs.west + configs.resolution_x * configs.columns as f64;

    // read the projection file
    let prj_file = Path::new(&file_name)
        .with_extension("prj")
        .into_os_string()
        .into_string()
        .expect("Error creating projection file name for BIL file.");

    if std::path::Path::new(&prj_file).exists() {
        let f = File::open(prj_file.clone()).expect("Error opening BIL projection (PRJ) file.");
        let f = BufReader::new(f);
        configs.projection = String::new();
        for line in f.lines() {
            let line_unwrapped = line.unwrap();
            if !line_unwrapped.is_empty() {
                configs.projection = format!("{}{}\n", configs.projection, line_unwrapped);
            }
        }
    }

    // read the data file
    data.reserve(configs.rows * configs.columns);

    let data_file = Path::new(&file_name)
        .with_extension("bil")
        .into_os_string()
        .into_string()
        .expect("Error creating file name string for BIL file.");
    let mut f = File::open(data_file.clone()).expect("Error opening BIL data file.");

    configs.minimum = f64::INFINITY;
    configs.maximum = f64::NEG_INFINITY;

    let mut data_size = 4;
    let mut z: f64;
    match configs.data_type {
        DataType::U8 => {
            data_size = 1;
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = buffer[offset] as f64;
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        DataType::U16 => {
            data_size = 2;
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = if configs.endian == Endianness::LittleEndian {
                        u16::from_le_bytes(get_two_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    } else {
                        u16::from_be_bytes(get_two_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    };
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        DataType::U32 => {
            data_size = 4;
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = if configs.endian == Endianness::LittleEndian {
                        u32::from_le_bytes(get_four_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    } else {
                        u32::from_be_bytes(get_four_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    };
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        DataType::I8 => {
            data_size = 1;
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = (buffer[offset] as i8) as f64;
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        DataType::I16 => {
            data_size = 2;
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = if configs.endian == Endianness::LittleEndian {
                        i16::from_le_bytes(get_two_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    } else {
                        i16::from_be_bytes(get_two_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    };
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        DataType::I32 => {
            data_size = 4;
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = if configs.endian == Endianness::LittleEndian {
                        i32::from_le_bytes(get_four_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    } else {
                        i32::from_be_bytes(get_four_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    };
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        DataType::F32 => {
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = if configs.endian == Endianness::LittleEndian {
                        f32::from_le_bytes(get_four_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    } else {
                        f32::from_be_bytes(get_four_bytes(&buffer[offset..offset + data_size]))
                            as f64
                    };
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        DataType::F64 => {
            data_size = 8;
            for row in 0..configs.rows as u64 {
                let mut buffer = vec![0; band_row_bytes];
                f.seek(SeekFrom::Start(row * total_row_bytes))
                    .expect("Error while seeking to position in BIL file.");
                f.read(&mut buffer)
                    .expect("Error while reading from BIL data file.");

                let mut offset: usize;
                for col in 0..configs.columns {
                    offset = col * data_size;
                    z = if configs.endian == Endianness::LittleEndian {
                        f64::from_le_bytes(get_eight_bytes(&buffer[offset..offset + data_size]))
                    } else {
                        f64::from_be_bytes(get_eight_bytes(&buffer[offset..offset + data_size]))
                    };
                    data.push(z);

                    if z != configs.nodata {
                        if z < configs.minimum {
                            configs.minimum = z;
                        }
                        if z > configs.maximum {
                            configs.maximum = z;
                        }
                    }
                }
            }
        }
        _ => {
            panic!("Unsupported BIL data type.");
        }
    }

    Ok(())
}

fn get_two_bytes(buf: &[u8]) -> [u8; 2] {
    buf.try_into()
        .expect("Error: Slice with incorrect length specified to get_two_bytes.")
}

fn get_four_bytes(buf: &[u8]) -> [u8; 4] {
    buf.try_into()
        .expect("Error: Slice with incorrect length specified to get_four_bytes.")
}

fn get_eight_bytes(buf: &[u8]) -> [u8; 8] {
    buf.try_into()
        .expect("Error: Slice with incorrect length specified to get_eight_bytes.")
}

pub fn write_esri_bil<'a>(r: &'a mut Raster) -> Result<(), Error> {
    if r.configs.photometric_interp == PhotometricInterpretation::RGB {
        panic!(
            "Single-band Esri BIL files are not suitable for storing RGB data. WhiteboxTools 
        presently only supports single-band BIL files. Use a GeoTiff format instead."
        );
    }

    /*
        Save the header file.

        The following is an example of the header file (HDR):

        BYTEORDER      I
        LAYOUT         BIL
        NROWS          5016
        NCOLS          8500
        NBANDS         1
        NBITS          32
        BANDROWBYTES   34000
        TOTALROWBYTES  34000
        PIXELTYPE      FLOAT
        ULXMAP         492088.919783702
        ULYMAP         4737707.84705645
        XDIM           0.499959505373639
        YDIM           0.499912454789739
        NODATA         -3.4028231e+38
    */
    let header_file = Path::new(&r.file_name)
        .with_extension("hdr")
        .into_os_string()
        .into_string()
        .expect("Error when trying to create BIL header (HDR) file.");

    let f = File::create(header_file).expect("Error while creating BIL file.");
    let mut writer = BufWriter::new(f);

    if r.configs.endian == Endianness::LittleEndian {
        writer
            .write_all("BYTEORDER      I\n".as_bytes())
            .expect("Error while writing to BIL file.");
    } else {
        writer
            .write_all("BYTEORDER      M\n".as_bytes())
            .expect("Error while writing to BIL file.");
    }

    writer
        .write_all("LAYOUT         BIL\n".as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("NROWS          {}\n", r.configs.rows);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("NCOLS          {}\n", r.configs.columns);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    writer
        .write_all("NBANDS         1\n".as_bytes())
        .expect("Error while writing to BIL file.");

    let nbits: usize;
    let pixel_type: String;
    match r.configs.data_type {
        DataType::U8 => {
            nbits = 8;
            pixel_type = "UNSIGNEDINT".to_string();
        }
        DataType::U16 => {
            nbits = 16;
            pixel_type = "UNSIGNEDINT".to_string();
        }
        DataType::U32 => {
            nbits = 32;
            pixel_type = "UNSIGNEDINT".to_string();
        }
        DataType::I8 => {
            nbits = 8;
            pixel_type = "SIGNEDINT".to_string();
        }
        DataType::I16 => {
            nbits = 16;
            pixel_type = "SIGNEDINT".to_string();
        }
        DataType::I32 => {
            nbits = 32;
            pixel_type = "SIGNEDINT".to_string();
        }
        DataType::F32 => {
            nbits = 32;
            pixel_type = "FLOAT".to_string();
        }
        DataType::F64 => {
            nbits = 64;
            pixel_type = "FLOAT".to_string();
        }
        _ => panic!("The raster is of a data type that is not supported by the BIL raster format."),
    }

    let s = format!("NBITS          {}\n", nbits);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("BANDROWBYTES   {}\n", nbits / 8 * r.configs.columns);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("TOTALROWBYTES  {}\n", nbits / 8 * r.configs.columns);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("PIXELTYPE      {}\n", pixel_type);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!(
        "ULXMAP         {}\n",
        r.configs.west + r.configs.resolution_x / 2.0
    );
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!(
        "ULYMAP         {}\n",
        r.configs.north - r.configs.resolution_y / 2.0
    );
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("XDIM           {}\n", r.configs.resolution_x);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("YDIM           {}\n", r.configs.resolution_y);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let s = format!("NODATA         {}\n", r.configs.nodata);
    writer
        .write_all(s.as_bytes())
        .expect("Error while writing to BIL file.");

    let _ = writer.flush();

    // output the projection file
    if r.configs.projection != "not specified" {
        let prj_file = Path::new(&r.file_name)
            .with_extension("prj")
            .into_os_string()
            .into_string()
            .expect("Error when trying to create BIL projection (PRJ) file.");
        let f = File::create(&prj_file)?;
        let mut writer = BufWriter::new(f);

        writer
            .write_all(r.configs.projection.as_bytes())
            .expect("Error while writing to BIL file.");

        let _ = writer.flush();
    }

    // write the data file
    let data_file = Path::new(&r.file_name)
        .with_extension("bil")
        .into_os_string()
        .into_string()
        .expect("Error when trying to create BIL file.");
    let f = File::create(&data_file)?;
    let mut writer = BufWriter::new(f);

    match r.configs.data_type {
        DataType::U8 => {
            for i in 0..r.data.len() {
                writer
                    .write(&([r.data[i] as u8]))
                    .expect("Error writing bytes to BIL file.");
            }
        }
        DataType::U16 => {
            for i in 0..r.data.len() {
                writer
                    .write(&(r.data[i] as u16).to_le_bytes())
                    .expect("Error writing bytes to BIL file.");
            }
        }
        DataType::U32 => {
            for i in 0..r.data.len() {
                writer
                    .write(&(r.data[i] as u32).to_le_bytes())
                    .expect("Error writing bytes to BIL file.");
            }
        }
        DataType::I8 => {
            for i in 0..r.data.len() {
                writer
                    .write(&(r.data[i] as i8).to_le_bytes())
                    .expect("Error writing bytes to BIL file.");
            }
        }
        DataType::I16 => {
            for i in 0..r.data.len() {
                writer
                    .write(&(r.data[i] as i16).to_le_bytes())
                    .expect("Error writing bytes to BIL file.");
            }
        }
        DataType::I32 => {
            for i in 0..r.data.len() {
                writer
                    .write(&(r.data[i] as i32).to_le_bytes())
                    .expect("Error writing bytes to BIL file.");
            }
        }
        DataType::F32 => {
            for i in 0..r.data.len() {
                writer
                    .write(&(r.data[i] as f32).to_le_bytes())
                    .expect("Error writing bytes to BIL file.");
            }
        }
        DataType::F64 => {
            for i in 0..r.data.len() {
                writer
                    .write(&(r.data[i]).to_le_bytes())
                    .expect("Error writing bytes to BIL file.");
            }
        }
        _ => panic!("The raster is of a data type that is not supported by the BIL raster format."),
    }

    let _ = writer.flush();

    Ok(())
}
