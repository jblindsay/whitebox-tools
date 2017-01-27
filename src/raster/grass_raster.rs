use std::io::Error;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;
use std::f64;
use std::fs::File;
use raster::*;

pub fn read_grass_raster(file_name: &String, configs: &mut RasterConfigs, data: &mut Vec<f64>) -> Result<(), Error> {
    // read the file
    let f = try!(File::open(file_name));
    let f = BufReader::new(f);

    //let mut likely_float = false;
    let mut multiplier = 1.0;
    let mut null_str = String::from("");
    let mut null_is_str = false;
    for line in f.lines() {
        let line_unwrapped = line.unwrap();
        let line_split = line_unwrapped.split(":");
        let vec = line_split.collect::<Vec<&str>>();
        if vec[0].to_lowercase().contains("rows") {
            configs.rows = vec[1].trim().to_string().parse::<usize>().unwrap();
        } else if vec[0].to_lowercase().contains("cols") {
            configs.columns = vec[1].trim().to_string().parse::<usize>().unwrap();
        } else if vec[0].to_lowercase().contains("north") {
            configs.north = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("south") {
            configs.south = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("east") {
            configs.east = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("west") {
            configs.west = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("cellsize") {
            configs.resolution_x = vec[1].trim().to_string().parse::<f64>().unwrap();
            configs.resolution_y = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("null") {
            if is_number(vec[1].trim().to_string()) {
                configs.nodata = vec[1].trim().to_string().parse::<f64>().unwrap();
                if vec[1].contains(".") {
                    //likely_float = true;
                    configs.data_type = DataType::F32;
                } else {
                    configs.data_type = DataType::I32;
                }
            } else {
                null_is_str = true;
                null_str = vec[1].trim().to_string();
                configs.nodata = 32768.0f64;
            }
        } else if vec[0].to_lowercase().contains("type") {
            if vec[1].contains("float") {
                //likely_float = true;
                configs.data_type = DataType::F32;
            } if vec[1].contains("double") {
                //likely_float = true;
                configs.data_type = DataType::F64;
            } else {
                configs.data_type = DataType::I32;
            }
            configs.nodata = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else if vec[0].to_lowercase().contains("multiplier") {
            multiplier = vec[1].trim().to_string().parse::<f64>().unwrap();
        } else { // it's a data line
            if !null_is_str {
                let mut val_num;
                for val in vec {
                    val_num = val.trim().to_string().parse::<f64>().unwrap();
                    if val_num != configs.nodata {
                        data.push(val_num * multiplier);
                    } else {
                        data.push(val_num);
                    }
                }
            } else {
                let mut val_string;
                for val in vec {
                    val_string = val.trim().to_string();
                    if val_string != null_str {
                        data.push(val.trim().to_string().parse::<f64>().unwrap() * multiplier);
                    } else {
                        data.push(configs.nodata);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn write_grass_raster<'a>(r: &'a mut Raster) -> Result<(), Error> {

    // Save the file
    let f = try!(File::create(&(r.file_name)));
    let mut writer = BufWriter::new(f);

    let s = format!("north:                   {}\n", &format!("{:.*} ", 2, r.configs.north));
    try!(writer.write_all(s.as_bytes()));

    let s = format!("south:                   {}\n", &format!("{:.*} ", 2, r.configs.south));
    try!(writer.write_all(s.as_bytes()));

    let s = format!("east:                    {}\n", &format!("{:.*} ", 2, r.configs.east));
    try!(writer.write_all(s.as_bytes()));

    let s = format!("west:                    {}\n", &format!("{:.*} ", 2, r.configs.west));
    try!(writer.write_all(s.as_bytes()));

    let s = format!("rows:                    {}\n", r.configs.rows);
    try!(writer.write_all(s.as_bytes()));

    let s = format!("cols:                    {}\n", r.configs.columns);
    try!(writer.write_all(s.as_bytes()));

    if r.configs.data_type == DataType::F32 || r.configs.data_type == DataType::F64 {
        let s = format!("null:                    {}\n", &format!("{:.*} ", 2, r.configs.nodata));
        try!(writer.write_all(s.as_bytes()));
    } else {
        let s = format!("null:                    {}\n", &format!("{:.*} ", 0, r.configs.nodata));
        try!(writer.write_all(s.as_bytes()));
    }

    if r.configs.data_type == DataType::F32 {
        let s = format!("type:                    float\n");
        try!(writer.write_all(s.as_bytes()));
    } else if r.configs.data_type == DataType::F64 {
        let s = format!("type:                    double\n");
        try!(writer.write_all(s.as_bytes()));
    } else {
        let s = format!("type:                    int\n");
        try!(writer.write_all(s.as_bytes()));
    }

    try!(writer.write_all("".as_bytes()));

    // write the data
    let mut s2 = String::new();
    let num_cells: usize = r.configs.rows * r.configs.columns;
    let mut col = 0;
    if r.configs.data_type == DataType::F32 || r.configs.data_type == DataType::F64 {
        for i in 0..num_cells {
            if col < r.configs.columns - 1 {
                s2 += &format!("{:.*} ", 2, r.data[i]);
            } else {
                s2 += &format!("{:.*}\n", 2, r.data[i]);
            }
            col += 1;
            if col == r.configs.columns {
                try!(writer.write_all(s2.as_bytes()));
                s2 = String::new();
                col = 0;
            }
        }
    } else {
        for i in 0..num_cells {
            if col < r.configs.columns - 1 {
                s2 += &format!("{:.*} ", 0, r.data[i]);
            } else {
                s2 += &format!("{:.*}\n", 0, r.data[i]);
            }
            col += 1;
            if col == r.configs.columns {
                try!(writer.write_all(s2.as_bytes()));
                s2 = String::new();
                col = 0;
            }
        }
    }

    let _ = writer.flush();

    Ok(())
}

#[inline]
fn is_number(value: String) -> bool {
    value.parse::<f64>().is_ok()
}
