// extern crate byteorder;

pub mod arcascii_raster;
pub mod arcbinary_raster;
pub mod geotiff;
pub mod grass_raster;
pub mod idrisi_raster;
pub mod saga_raster;
pub mod surfer7_raster;
pub mod surfer_ascii_raster;
pub mod whitebox_raster;

use std::io::Error;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::io::BufReader;
// use std::io::BufRead;
use std::default::Default;
use std::fs::File;
use std::path::Path;
use std::f64;
use raster::arcascii_raster::*;
use raster::arcbinary_raster::*;
use raster::geotiff::*;
use raster::grass_raster::*;
use raster::idrisi_raster::*;
use raster::saga_raster::*;
use raster::surfer7_raster::*;
use raster::surfer_ascii_raster::*;
use raster::whitebox_raster::*;
use io::byte_order_reader::*;
use std::ops::{Index, IndexMut};

#[derive(Default, Clone)]
pub struct Raster {
    pub file_name: String,
    file_mode: String,
    pub raster_type: RasterType,
    pub configs: RasterConfigs,
    data: Vec<f64>,
}

impl Index<(isize, isize)> for Raster {
    type Output = f64;

    fn index<'a>(&'a self, index: (isize, isize)) -> &'a f64 {
        let row = index.0;
        let column = index.1;

        if column < 0 { return &self.configs.nodata; }
        if row < 0 { return &self.configs.nodata; }

        let c: usize = column as usize;
        let r: usize = row as usize;

        if c >= self.configs.columns { return &self.configs.nodata; }
        if r >= self.configs.rows { return &self.configs.nodata; }
        let idx: usize = r * self.configs.columns + c;
        &self.data[idx]
    }
}

impl IndexMut<(isize, isize)> for Raster {
    fn index_mut<'a>(&'a mut self, index: (isize, isize)) -> &'a mut f64 {
        let row = index.0;
        let column = index.1;
        if column < 0 { return &mut self.configs.nodata; }
        if row < 0 { return &mut self.configs.nodata; }
        let c: usize = column as usize;
        let r: usize = row as usize;
        if c >= self.configs.columns { return &mut self.configs.nodata; }
        if r >= self.configs.rows { return &mut self.configs.nodata; }
        let idx = r * self.configs.columns + c;
        &mut self.data[idx as usize]
    }
}

impl Raster {
    pub fn new<'a>(file_name: &'a str, file_mode: &'a str) -> Result<Raster, Error> {
        let fm: String = file_mode.to_lowercase();
        let mut r = Raster {
            file_name: file_name.to_string(),
            file_mode: fm.clone(),
            raster_type: get_raster_type_from_file(file_name.to_string(), fm.clone()),
            ..Default::default()
        };
        if r.file_mode == "r" {
            match get_raster_type_from_file(file_name.to_string(), fm) {
                RasterType::ArcBinary => {
                    let _ = read_arcbinary(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::ArcAscii => {
                    let _ = read_arcascii(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::GeoTiff => {
                    let _ = read_geotiff(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::GrassAscii => {
                    let _ = read_grass_raster(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::IdrisiBinary => {
                    let _ = read_idrisi(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::SagaBinary => {
                    let _ = read_saga(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::Surfer7Binary => {
                    let _ = read_surfer7(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::SurferAscii => {
                    let _ = read_surfer_ascii_raster(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::Whitebox => {
                    let _ = read_whitebox(&r.file_name, &mut r.configs, &mut r.data).unwrap();
                    return Ok(r);
                },
                RasterType::Unknown => { return Err(Error::new(ErrorKind::Other, "Unrecognized raster type")); },
            }
        } else { // write

        }
        Err(Error::new(ErrorKind::Other, "Error creating raster"))
    }

    pub fn initialize_using_file<'a>(file_name: &'a str, input: &'a Raster) -> Raster {
        let mut output = Raster { file_name: file_name.to_string(), ..Default::default() };
        output.file_mode = "w".to_string();
        output.raster_type = get_raster_type_from_file(file_name.to_string(), "w".to_string());
        output.configs.rows = input.configs.rows;
        output.configs.columns = input.configs.columns;
        output.configs.north = input.configs.north;
        output.configs.south = input.configs.south;
        output.configs.east = input.configs.east;
        output.configs.west = input.configs.west;
        output.configs.resolution_x = input.configs.resolution_x;
        output.configs.resolution_y = input.configs.resolution_y;
        output.configs.nodata = input.configs.nodata;
        output.configs.data_type = input.configs.data_type;
        output.configs.photometric_interp = input.configs.photometric_interp;
        output.configs.palette = input.configs.palette.clone();
        output.configs.projection = input.configs.projection.clone();
        output.configs.xy_units = input.configs.xy_units.clone();
        output.configs.z_units = input.configs.z_units.clone();
        output.configs.endian = input.configs.endian.clone();
        output.configs.palette_nonlinearity = input.configs.palette_nonlinearity;
        output.configs.pixel_is_area = input.configs.pixel_is_area;
    	output.configs.epsg_code = input.configs.epsg_code;
        output.configs.coordinate_ref_system_wkt = input.configs.coordinate_ref_system_wkt.clone();

        if output.raster_type == RasterType::SurferAscii ||
            output.raster_type == RasterType::Surfer7Binary {
            output.configs.nodata = 1.71041e38;
        }

        output.data = vec![output.configs.nodata; output.configs.rows * output.configs.columns];

        output
    }

    pub fn get_value(&self, row: isize, column: isize) -> f64 {
        if column < 0 { return self.configs.nodata; }
        if row < 0 { return self.configs.nodata; }

        let c: usize = column as usize;
        let r: usize = row as usize;

        if c >= self.configs.columns { return self.configs.nodata; }
        if r >= self.configs.rows { return self.configs.nodata; }
        let idx: usize = r * self.configs.columns + c;
        self.data[idx]
    }

    pub fn set_value(&mut self, row: isize, column: isize, value: f64) {
        if column >= 0 && row >= 0 {
            let c: usize = column as usize;
            let r: usize = row as usize;
            if c < self.configs.columns && r < self.configs.rows {
                let idx = r * self.configs.columns + c;
                self.data[idx] = value;
            }
        }
    }

    pub fn write(&mut self) -> Result<(), Error> {
        match self.raster_type {
            RasterType::ArcAscii => {
                let _ = match write_arcascii(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::ArcBinary => {
                let _ = match write_arcbinary(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::GeoTiff => {
                let _ = match write_geotiff(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::GrassAscii => {
                let _ = match write_grass_raster(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::IdrisiBinary => {
                let _ = match write_idrisi(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::SagaBinary => {
                let _ = match write_saga(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::Surfer7Binary => {
                let _ = match write_surfer7(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::SurferAscii => {
                let _ = match write_surfer_ascii_raster(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::Whitebox => {
                let _ = match write_whitebox(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            },
            RasterType::Unknown => { return Err(Error::new(ErrorKind::Other, "Unrecognized raster type")); },
        }
        Ok(())
    }

    pub fn add_metadata_entry(&mut self, value: String) {
        self.configs.metadata.push(value);
    }

    pub fn get_metadata_entry(&self, idx: usize) -> String {
        if idx < self.configs.metadata.len() {
            return self.configs.metadata.get(idx).unwrap().clone();
        }
        String::new()
    }

    pub fn is_in_geographic_coordinates(&self) -> bool {
        if self.configs.epsg_code == 4322 || self.configs.epsg_code == 4326 ||
            self.configs.epsg_code == 4629 || self.configs.epsg_code == 4277 {
            return true;
        }
        let wkt = self.configs.coordinate_ref_system_wkt.to_lowercase();
        if !wkt.contains("projcs[") {
            return true;
        }
        if self.configs.xy_units.to_lowercase().contains("deg") {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct RasterConfigs {
    pub title: String,
    pub rows: usize,
    pub columns: usize,
    pub bands: u8,
    pub nodata: f64,
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
    pub resolution_x: f64,
    pub resolution_y: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub display_min: f64,
    pub display_max: f64,
    pub palette: String,
    pub projection: String,
    pub endian: Endianness,
    pub photometric_interp: PhotometricInterpretation,
    pub data_type: DataType,
    pub palette_nonlinearity: f64,
    pub z_units: String,
    pub xy_units: String,
    pub reflect_at_edges: bool,
	pub pixel_is_area: bool,
	pub epsg_code: u16,
    pub coordinate_ref_system_wkt: String,
    pub metadata: Vec<String>,
}

impl Default for RasterConfigs {
    fn default() -> RasterConfigs {
        RasterConfigs {
            title: String::from(""),
            bands: 1,
            rows: 0,
            columns: 0,
            nodata: -32768.0,
            north: f64::NEG_INFINITY,
            south: f64::INFINITY,
            east: f64::NEG_INFINITY,
            west: f64::INFINITY,
            resolution_x: f64::NEG_INFINITY,
            resolution_y: f64::NEG_INFINITY,
            minimum: f64::INFINITY,
            maximum: f64::NEG_INFINITY,
            display_min: f64::INFINITY,
            display_max: f64::NEG_INFINITY,
            palette: "not specified".to_string(),
            projection: "not specified".to_string(),
            endian: Endianness::LittleEndian,
            photometric_interp: PhotometricInterpretation::Unknown,
            data_type: DataType::Unknown,
            palette_nonlinearity: -1.0,
            z_units: "not specified".to_string(),
            xy_units: "not specified".to_string(),
            reflect_at_edges: false,
            pixel_is_area: true,
            epsg_code: 0u16,
            coordinate_ref_system_wkt: "not specified".to_string(),
            metadata: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RasterType {
    Unknown,
    ArcAscii,
    ArcBinary,
    GeoTiff,
    GrassAscii,
    IdrisiBinary,
    SagaBinary,
    Surfer7Binary,
    SurferAscii,
    Whitebox
    // EsriBIL,
}

impl Default for RasterType {
    fn default() -> RasterType { RasterType::Unknown }
}


#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DataType {
    F64, F32, I64, I32, I16, I8, U64, U32, U16, U8, RGB24, RGB48, RGBA32, Unknown
}

impl Default for DataType {
    fn default() -> DataType { DataType::Unknown }
}

impl DataType {
    pub fn get_data_size(&self) -> usize {
        match *self {
            DataType::F64 => 8usize,
            DataType::F32 => 4usize,
            DataType::I64 => 8usize,
            DataType::I32 => 4usize,
            DataType::I16 => 2usize,
            DataType::I8 => 1usize,
            DataType::U64 => 8usize,
            DataType::U32 => 4usize,
            DataType::U16 => 2usize,
            DataType::U8 => 1usize,
            DataType::RGB24 => 3usize,
            DataType::RGB48 => 6usize,
            DataType::RGBA32 => 4usize,
            DataType::Unknown => 0usize,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PhotometricInterpretation {
    Continuous,
    Categorical,
    Boolean,
    RGB,
    Paletted,
    // Rgb32,
    // Rgb24,
    Unknown
}

impl Default for PhotometricInterpretation {
    fn default() -> PhotometricInterpretation { PhotometricInterpretation::Unknown }
}

fn get_raster_type_from_file(file_name: String, file_mode: String) -> RasterType {
    // get the file extension
    let extension: String = match Path::new(&file_name).extension().unwrap().to_str() {
        Some(n) => n.to_string().to_lowercase(),
        None => "".to_string(),
    };

    if extension == "tas" || extension == "dep" {
        return RasterType::Whitebox;
    } else if extension == "tif" || extension == "tiff" {
        return RasterType::GeoTiff;
    } else if extension == "flt" {
        return RasterType::ArcBinary;
    } else if extension == "rdc" || extension == "rst" {
        return RasterType::IdrisiBinary;
    } else if extension == "sdat" || extension == "sgrd" {
        return RasterType::SagaBinary;
    } else if extension == "grd" {
        if file_mode == "r" {
            // It could be a SurferAscii or a Surfer7Binary.
            let mut f = File::open(file_name).unwrap();
            let mut buffer = [0; 4];
            f.read_exact(&mut buffer).unwrap();
            //let small_chunk = String::from_utf8_lossy(&buffer[0..8]).to_string();
            //if small_chunk.contains("DSAA") {
            if buffer[0] == 68 && buffer[1] == 83 && buffer[2] == 65 && buffer[3] == 65  { // DSAA signature
                return RasterType::SurferAscii;
            } else {
                return RasterType::Surfer7Binary;
            }
        }
        return RasterType::Surfer7Binary;
    } else if extension == "asc" || extension == "txt" || extension == "" {
        // what mode is this raster in?
        if file_mode == "r" {
            // It could be an ArcAscii or a GrassAscii.
            let f = File::open(file_name).unwrap();
            let file = BufReader::new(&f);
            let mut line_count = 0;
            for line in file.lines() {
                let l = line.unwrap();
                if l.contains("north") || l.contains("south") || l.contains("east") || l.contains("west") {
                    return RasterType::GrassAscii;
                }
                if l.contains("xllcorner") || l.contains("yllcorner") || l.contains("xllcenter") || l.contains("yllcenter") {
                    return RasterType::ArcAscii;;
                }
                if line_count > 7 {
                    break;
                }
                line_count += 1;
            }
        }
        // For a file_mode "w", there is not way of knowing if it is an Arc or GRASS ASCII raster.
        // Default to ArcAscii.
        return RasterType::ArcAscii;
    }

    RasterType::Unknown
}

// #[derive(Debug, Copy, Clone, PartialEq)]
// pub enum RasterByteOrder {
//     LittleEndian,
//     BigEndian,
// }
//
// impl Default for RasterByteOrder {
//     fn default() -> RasterByteOrder { RasterByteOrder::LittleEndian }
// }
//
// impl RasterByteOrder {
//     pub fn from_str<'a>(val: &'a str) -> RasterByteOrder {
//         let val_lc: &str = &val.to_lowercase();
//         if val_lc.contains("lsb") || val_lc.contains("little") || val_lc.contains("intel") {
//             return RasterByteOrder::LittleEndian;
//         } else {
//             return RasterByteOrder::BigEndian;
//         }
//     }
// }

// #[derive(Debug, Copy, Clone)]
// pub struct ByteOrderWriter {
//     native_endianness: RasterByteOrder,
//     byte_order: RasterByteOrder,
// }
//
// impl ByteOrderWriter {
//     fn new(byte_order: RasterByteOrder) -> ByteOrderWriter {
//         let ne;
//         if cfg!(target_endian = "little") {
//             ne = RasterByteOrder::LittleEndian;
//         } else {
//             ne = RasterByteOrder::BigEndian;
//         }
//         ByteOrderWriter { byte_order: byte_order, native_endianness: ne }
//     }
//
// }
