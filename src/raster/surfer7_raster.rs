use std::io::Error;
use std::io::ErrorKind;
use std::io::BufWriter;
use std::io::prelude::*;
use std::f64;
use std::fs::File;
use std::fs;
use std::mem;
use raster::*;

pub fn read_surfer7(file_name: &String, configs: &mut RasterConfigs, data: &mut Vec<f64>) -> Result<(), Error> {

    // read data file
    let mut f = try!(File::open(file_name.clone()));
    let metadata = try!(fs::metadata(file_name.clone()));
    let file_size: usize = metadata.len() as usize;
    let mut buffer = vec![0; file_size];

    // read the file's bytes into a buffer
    try!(f.read(&mut buffer));

    let mut offset = 0;

    // read the header component
    let header_id = unsafe { mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]) };
    if header_id != 0x42525344 {
        return Err(Error::new(ErrorKind::InvalidData, "The input  Surfer does not appear to be formated correctly."));
    }
    offset += 4;

    let header_sz = unsafe { mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]) };
    if header_sz != 4 {
        return Err(Error::new(ErrorKind::InvalidData, "The input  Surfer does not appear to be formated correctly."));
    }
    offset += 4;

    let version = unsafe { mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]) };
    offset += 4;

    // read the grid component
    let grid_id = unsafe { mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]) };
    if grid_id != 0x44495247 {
        return Err(Error::new(ErrorKind::InvalidData, "The input  Surfer does not appear to be formated correctly."));
    }
    offset += 4;

    let grid_sz = unsafe { mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]) };
    if grid_sz != 72 {
        return Err(Error::new(ErrorKind::InvalidData, "The input  Surfer does not appear to be formated correctly."));
    }
    offset += 4;

    configs.rows = unsafe { mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]) } as usize;
    offset += 4;

    configs.columns = unsafe { mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]) } as usize;
    offset += 4;

    configs.west =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };
    offset += 8;

    configs.south =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };
    offset += 8;

    configs.resolution_x =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };
    offset += 8;

    configs.resolution_y =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };
    offset += 8;

    configs.east = configs.west + configs.resolution_x * configs.columns as f64;
    configs.north = configs.south + configs.resolution_x * configs.rows as f64;

    configs.minimum =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };
    offset += 8;

    configs.maximum =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };
    offset += 8;

    // Read the rotation value. This isn't actually used and should be set to zero. Notice that the offical
    // documentation on the Golden Software site does not list a rotation value in the description of the
    // grid section and only in the example that they provide. This is ambiguous and could cause compatibility
    // issues.
    let rotation_value =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };
    offset += 8;
    if rotation_value != 0.0f64 {
        println!("Warning, non-zero rotation values are not currently supported.");
    }

    configs.nodata =  unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2],
                    buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]) };

    configs.data_type = DataType::F64;

    let num_cells = configs.rows * configs.columns;
    data.clear();
    for _ in 0..num_cells {
        data.push(configs.nodata);
    }

    if version == 2 {
        let mut i: usize;
        let mut value: f64;
        for row in (0..configs.rows).rev() {
            for col in 0..configs.columns {
                i = row * configs.columns + col;
                value = unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1],
                                    buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5],
                                    buffer[offset+6], buffer[offset+7]]) };
                if value != configs.nodata {
                    data[i] = value;
                } else {
                    data[i] = configs.nodata;
                }
            }
        }
    } else {
        let mut i: usize;
        let mut value: f64;
        for row in (0..configs.rows).rev() {
            for col in 0..configs.columns {
                i = row * configs.columns + col;
                value = unsafe { mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1],
                                    buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5],
                                    buffer[offset+6], buffer[offset+7]]) };
                if value <= configs.nodata {
                    data[i] = value;
                } else {
                    data[i] = configs.nodata;
                }
            }
        }
    }

    Ok(())
}

pub fn write_surfer7<'a>(r: &'a mut Raster) -> Result<(), Error> {
    // figure out the minimum and maximum values
    for val in &r.data {
        let v = *val;
        if v != r.configs.nodata {
            if v < r.configs.minimum { r.configs.minimum = v; }
            if v > r.configs.maximum { r.configs.maximum = v; }
        }
    }

    // Save the file
    let f = try!(File::create(r.file_name.clone()));
    let mut writer = BufWriter::new(f);

    let mut u32_bytes: [u8; 4];
    let mut u64_bytes: [u8; 8];

    u32_bytes = unsafe { mem::transmute(0x42525344i32) };
    try!(writer.write(&u32_bytes));

    u32_bytes = unsafe { mem::transmute(4i32) };
    try!(writer.write(&u32_bytes));

    u32_bytes = unsafe { mem::transmute(2i32) };
    try!(writer.write(&u32_bytes));

    u32_bytes = unsafe { mem::transmute(0x44495247i32) };
    try!(writer.write(&u32_bytes));

    u32_bytes = unsafe { mem::transmute(72i32) };
    try!(writer.write(&u32_bytes));

    u32_bytes = unsafe { mem::transmute(r.configs.rows as i32) };
    try!(writer.write(&u32_bytes));

    u32_bytes = unsafe { mem::transmute(r.configs.columns as i32) };
    try!(writer.write(&u32_bytes));

    u64_bytes = unsafe { mem::transmute(r.configs.west) };
    try!(writer.write(&u64_bytes));

    u64_bytes = unsafe { mem::transmute(r.configs.south) };
    try!(writer.write(&u64_bytes));

    u64_bytes = unsafe { mem::transmute(r.configs.resolution_x) };
    try!(writer.write(&u64_bytes));

    u64_bytes = unsafe { mem::transmute(r.configs.resolution_y) };
    try!(writer.write(&u64_bytes));

    u64_bytes = unsafe { mem::transmute(r.configs.minimum) };
    try!(writer.write(&u64_bytes));

    u64_bytes = unsafe { mem::transmute(r.configs.maximum) };
    try!(writer.write(&u64_bytes));

    u64_bytes = unsafe { mem::transmute(0.0f64) }; // rotation of 0.0
    try!(writer.write(&u64_bytes));

    u64_bytes = unsafe { mem::transmute(1.70141e38f64) };
    try!(writer.write(&u64_bytes));

    // write the data
    u32_bytes = unsafe { mem::transmute(0x41544144i32) };
    try!(writer.write(&u32_bytes));

    u32_bytes = unsafe { mem::transmute((r.configs.rows * r.configs.columns * 8) as i32) };
    try!(writer.write(&u32_bytes));

    let mut i: usize;
    for row in (0..r.configs.rows).rev() {
        for col in 0..r.configs.columns {
            i = row * r.configs.columns + col;
            u64_bytes = unsafe { mem::transmute(r.data[i]) };
            try!(writer.write(&u64_bytes));
        }
    }

    let _ = writer.flush();

    Ok(())
}
