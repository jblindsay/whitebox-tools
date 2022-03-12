/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/01/2017
Last Modified: 07/12/2018
License: MIT
*/

#![allow(dead_code, unused_assignments)]
extern crate brotli;
extern crate las;
use super::header::LasHeader;
use super::point_data::{ ColourData, PointData, WaveformPacket };
use super::vlr::Vlr;
use super::zlidar_compression::{ZlidarCompression};
use whitebox_raster::geotiff::geokeys::GeoKeys;
use whitebox_common::spatial_ref_system::esri_wkt_from_epsg;
use whitebox_common::structures::{ BoundingBox, Point3D };
use whitebox_common::utils::{ ByteOrderReader, Endianness };
use byteorder::{ LittleEndian, WriteBytesExt };
use chrono::prelude::*;
use core::slice;
use miniz_oxide::deflate::compress_to_vec_zlib;
use miniz_oxide::inflate::decompress_to_vec_zlib;
// use std::collections::VecDeque;
// use std::cmp::min;
use std::f64;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufWriter, Cursor, Error, ErrorKind, Read, Seek};
use std::mem;
use std::ops::Index;
use std::path::Path;
use std::str;
use zip::read::{ZipArchive, ZipFile};
use zip::result::ZipResult;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
// use compression::prelude::*;
// use lz4_compression::prelude::compress;
use las::Reader;
use las::Read as OtherRead;
// use las::raw::point::Flags::{ThreeByte, TwoByte};
// use las::{Builder, Write, Writer};
use las::Builder;
use las::Write as OtherWrite;
use las::Writer as OtherWriter;
use las::raw::point::ScanAngle;
use las::raw::vlr::RecordLength;

#[derive(Default, Clone)]
pub struct LasFile {
    file_name: String,
    file_mode: String,
    pub header: LasHeader,
    pub vlr_data: Vec<Vlr>,
    point_data: Vec<PointData>,
    // point_buffer_size: usize,
    gps_data: Vec<f64>,
    colour_data: Vec<ColourData>,
    waveform_data: Vec<WaveformPacket>,
    pub geokeys: GeoKeys,
    pub wkt: String,
    // starting_point: usize,
    header_is_set: bool,
    pub use_point_intensity: bool,
    pub use_point_userdata: bool,
    pub compression: ZlidarCompression,
}

impl<'a> IntoIterator for &'a LasFile {
    type Item = &'a PointData;
    type IntoIter = slice::Iter<'a, PointData>;

    fn into_iter(self) -> slice::Iter<'a, PointData> {
        self.point_data.iter()
    }
}

impl Index<usize> for LasFile {
    type Output = PointData;

    fn index<'a>(&'a self, index: usize) -> &'a PointData {
        &self.point_data[index]
    }
}

impl LasFile {
    /// Constructs a new `LasFile` based on a file.
    /// The function takes the name of an existing raster file (`file_name`)
    /// and the `file_mode`, which can be 'r' (read), 'rh' (read header), and
    /// 'w' (write).
    pub fn new<'a>(file_name: &'a str, file_mode: &'a str) -> Result<LasFile, Error> {
        //LasFile {
        let mut lf = LasFile {
            file_name: file_name.to_string(),
            wkt: String::new(),
            ..Default::default()
        };
        lf.file_mode = file_mode.to_lowercase();
        if lf.file_mode == "r" || lf.file_mode == "rh" {
            lf.read()?;
        } else {
            lf.file_mode = "w".to_string();
        }
        // lf.point_buffer_size = 1000000;
        lf.use_point_intensity = true;
        lf.use_point_userdata = true;
        Ok(lf)
    }

    /// This function returns a new `LasFile` that has been initialized using another
    /// `LasFile`.
    /// Input Parameters:
    /// * file_name: The name of the LAS file to be created.
    /// * input: An existing LAS file.
    ///
    /// Output:
    /// * A LasFile struct, initialized with the header and VLRs of the input file.
    pub fn initialize_using_file<'a>(file_name: &'a str, input: &'a LasFile) -> LasFile {
        let mut output = LasFile {
            file_name: file_name.to_string(),
            wkt: String::new(),
            ..Default::default()
        };
        output.file_mode = "w".to_string();
        output.use_point_intensity = true;
        output.use_point_userdata = true;
        output.wkt = input.wkt.clone();
        output.compression = input.compression.clone();

        output.add_header(input.header.clone());

        // Copy the VLRs
        for i in 0..(input.header.number_of_vlrs as usize) {
            output.add_vlr(input.vlr_data[i].clone());
        }

        output
    }

    pub fn add_header(&mut self, header: LasHeader) {
        if self.file_mode == "r" {
            return;
        }
        self.header = header;

        self.header.number_of_vlrs = 0;
        self.header.number_of_points = 0;

        self.header.version_major = 1;
        self.header.version_minor = 3;
        // These must be set by the data
        self.header.min_x = f64::INFINITY;
        self.header.max_x = f64::NEG_INFINITY;
        self.header.min_y = f64::INFINITY;
        self.header.max_y = f64::NEG_INFINITY;
        self.header.min_z = f64::INFINITY;
        self.header.max_z = f64::NEG_INFINITY;

        self.header.system_id = "WhiteboxTools by John Lindsay   ".to_string();
        self.header.generating_software = "WhiteboxTools                   ".to_string();
        self.header.number_of_points_by_return_old = [0; 5];
        self.header.number_of_points_by_return = [0; 15];

        // self.header.x_scale_factor = 0.001;
        // self.header.y_scale_factor = 0.001;
        // self.header.z_scale_factor = 0.001;

        self.header_is_set = true;
    }

    pub fn add_vlr(&mut self, vlr: Vlr) {
        if self.file_mode == "r" {
            return;
        }
        // the header must be set before you can add VLRs
        if !self.header_is_set {
            panic!(
                "The header of a LAS file must be added before any VLRs. Please see add_header()."
            );
        }
        self.vlr_data.push(vlr);
        self.header.number_of_vlrs += 1;
    }

    pub fn add_point_record(&mut self, point: LidarPointRecord) {
        if self.file_mode == "r" {
            return;
        }
        if !self.header_is_set {
            panic!("The header of a LAS file must be added before any point records. Please see add_header().");
        }
        let mut which_return = 0_usize;
        let x: f64;
        let y: f64;
        let z: f64;
        match point {
            LidarPointRecord::PointRecord0 { 
                point_data 
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
            }
            LidarPointRecord::PointRecord1 {
                point_data,
                gps_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
            }
            LidarPointRecord::PointRecord2 {
                point_data,
                colour_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.colour_data.push(colour_data);
            }
            LidarPointRecord::PointRecord3 {
                point_data,
                gps_data,
                colour_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
            }
            LidarPointRecord::PointRecord4 {
                point_data,
                gps_data,
                wave_packet,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.waveform_data.push(wave_packet);
            }
            LidarPointRecord::PointRecord5 {
                point_data,
                gps_data,
                colour_data,
                wave_packet,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
                self.waveform_data.push(wave_packet);
            }
            LidarPointRecord::PointRecord6 {
                point_data,
                gps_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
            }
            LidarPointRecord::PointRecord7 {
                point_data,
                gps_data,
                colour_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
            }
            LidarPointRecord::PointRecord8 {
                point_data,
                gps_data,
                colour_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
            }
            LidarPointRecord::PointRecord9 {
                point_data,
                gps_data,
                wave_packet,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.waveform_data.push(wave_packet);
            }
            LidarPointRecord::PointRecord10 {
                point_data,
                gps_data,
                colour_data,
                wave_packet,
            } => {
                self.point_data.push(point_data);
                x = point_data.x as f64 * self.header.x_scale_factor + self.header.x_offset;
                y = point_data.y as f64 * self.header.y_scale_factor + self.header.y_offset;
                z = point_data.z as f64 * self.header.z_scale_factor + self.header.z_offset;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
                self.waveform_data.push(wave_packet);
            }
        }

        if x < self.header.min_x {
            self.header.min_x = x;
        }
        if x > self.header.max_x {
            self.header.max_x = x;
        }
        if y < self.header.min_y {
            self.header.min_y = y;
        }
        if y > self.header.max_y {
            self.header.max_y = y;
        }
        if z < self.header.min_z {
            self.header.min_z = z;
        }
        if z > self.header.max_z {
            self.header.max_z = z;
        }

        self.header.number_of_points += 1;
        if which_return == 0 {
            which_return = 1;
        }
        if which_return <= 5 {
            self.header.number_of_points_by_return[which_return - 1] += 1;
        }
    }

    pub fn get_record(&self, index: usize) -> LidarPointRecord {
        if index > self.point_data.len() {
            panic!("Index out of bounds.");
        }
        let lpr: LidarPointRecord;
        unsafe {
            // there's no need for all of the bounds checks that would come with regular indexing
            match self.header.point_format {
                0 => {
                    lpr = LidarPointRecord::PointRecord0 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                    };
                }
                1 => {
                    lpr = LidarPointRecord::PointRecord1 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                    };
                }
                2 => {
                    lpr = LidarPointRecord::PointRecord2 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        colour_data: *self.colour_data.get_unchecked(index), //[index],
                    };
                }
                3 => {
                    lpr = LidarPointRecord::PointRecord3 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                        colour_data: *self.colour_data.get_unchecked(index), //[index],
                    };
                }
                4 => {
                    lpr = LidarPointRecord::PointRecord4 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                        wave_packet: *self.waveform_data.get_unchecked(index), //[index],
                    };
                }
                5 => {
                    lpr = LidarPointRecord::PointRecord5 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                        colour_data: *self.colour_data.get_unchecked(index), //[index],
                        wave_packet: *self.waveform_data.get_unchecked(index), //[index],
                    };
                }
                6 => {
                    lpr = LidarPointRecord::PointRecord6 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                    };
                }
                7 => {
                    lpr = LidarPointRecord::PointRecord7 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                        colour_data: *self.colour_data.get_unchecked(index), //[index],
                    };
                }
                8 => {
                    lpr = LidarPointRecord::PointRecord8 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                        colour_data: *self.colour_data.get_unchecked(index), //[index],
                    };
                }
                9 => {
                    lpr = LidarPointRecord::PointRecord9 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                        wave_packet: *self.waveform_data.get_unchecked(index), //[index],
                    };
                }
                10 => {
                    lpr = LidarPointRecord::PointRecord10 {
                        point_data: *self.point_data.get_unchecked(index), //[index],
                        gps_data: *self.gps_data.get_unchecked(index),     //[index],
                        colour_data: *self.colour_data.get_unchecked(index), //[index],
                        wave_packet: *self.waveform_data.get_unchecked(index), //[index],
                    };
                }
                _ => {
                    panic!("Unsupported point format");
                }
            }
        }
        lpr
    }

    pub fn get_point_info(&self, index: usize) -> PointData {
        self.point_data[index]
    }

    pub fn get_transformed_coords(&self, index: usize) -> Point3D {
        let x = self.point_data[index].x as f64 * self.header.x_scale_factor + self.header.x_offset;
        let y = self.point_data[index].y as f64 * self.header.y_scale_factor + self.header.y_offset;
        let z = self.point_data[index].z as f64 * self.header.z_scale_factor + self.header.z_offset;

        Point3D::new(x, y, z)
    }

    pub fn get_rgb(&self, index: usize) -> Result<ColourData, Error> {
        if self.colour_data.len() >= index {
            return Ok(self.colour_data[index]);
        } else {
            return Err(Error::new(ErrorKind::NotFound, "RGB value not found, possibly because the file point format does not include colour data."));
        }
    }

    pub fn has_rgb(&self) -> bool {
        self.colour_data.len() > 0
    }

    pub fn get_gps_time(&self, index: usize) -> Option<f64> { // Result<f64, Error> {
        if self.gps_data.len() >= index {
            return Some(self.gps_data[index]); // Ok(self.gps_data[index]);
        // } else {
        //     return Err(Error::new(ErrorKind::NotFound, "GPS time value not found, possibly because the file point format does not include GPS data."));
        }
        None
    }

    pub fn has_gps_time(&self) -> bool {
        self.gps_data.len() > 0
    }

    pub fn get_short_filename(&self) -> String {
        let path = Path::new(&self.file_name);
        let file_name = path.file_stem().unwrap();
        let f = file_name.to_str().unwrap();
        f.to_string()
    }

    pub fn get_extent(&self) -> BoundingBox {
        BoundingBox {
            min_x: self.header.min_x,
            max_x: self.header.max_x,
            min_y: self.header.min_y,
            max_y: self.header.max_y,
        }
    }

    pub fn get_wkt(&mut self) -> String {
        if self.wkt.is_empty() {
            let epsg_code = self.geokeys.find_epsg_code();
            self.wkt = esri_wkt_from_epsg(epsg_code);
        }
        self.wkt.clone()
    }

    pub fn get_epsg_code(&self) -> u16 {
        self.geokeys.find_epsg_code()
    }

    pub fn read(&mut self) -> Result<(), Error> {
        if self.file_name.to_lowercase().ends_with(".zlidar") {
            return self.read_zlidar_data();
        }
        if self.file_name.to_lowercase().ends_with(".laz") {
            return self.read_laz_data();
        }
        let buffer = match self.file_name.to_lowercase().ends_with(".zip") {
            false => {
                let mut f = File::open(&self.file_name).expect("Error opening LAS file.");
                let metadata = fs::metadata(&self.file_name)?;
                // let file_size: usize = if self.file_mode != "rh" {
                //     metadata.len() as usize
                // } else {
                //     375 // the size of the header
                // };

                let file_size: usize = metadata.len() as usize;

                let mut buffer = vec![0; file_size]; // Vec::with_capacity(file_size);
                if file_size < 1024 * 1024 * 500 {
                    // 2147483646 is the actual maximum file read on Mac
                    f.read(&mut buffer)?;
                } else {
                    // let br = BufReader::new(f);
                    // let mut i = 0;
                    // for byte in br.bytes() {
                    //     buffer[i] = byte.unwrap();
                    //     i += 1;
                    // }

                    let block_size = 1024 * 1024 * 500;
                    let mut start_byte = 0usize;
                    let mut end_byte = block_size;
                    let mut bytes_read = 0;
                    while bytes_read < file_size {
                        f.read(&mut buffer[start_byte..end_byte])?;
                        start_byte += block_size;
                        end_byte += block_size;
                        if end_byte > file_size {
                            end_byte = file_size;
                        }
                        bytes_read += block_size;
                    }
                }

                buffer
            }
            true => {
                let file = File::open(&self.file_name)?;
                let mut zip = (zip::ZipArchive::new(file))?;
                let mut f = zip.by_index(0).unwrap();
                if !f.name().to_lowercase().ends_with(".las") {
                    return Err(Error::new(ErrorKind::InvalidData,
                "The data file contained within zipped archive does not have the proper 'las' extension."));
                }
                match f.compression() {
                    CompressionMethod::Stored | CompressionMethod::Deflated | CompressionMethod::Bzip2 => (),
                    _ => return Err(Error::new(ErrorKind::InvalidData,
                    "Either the file is formatted incorrectly or it is an unsupported compression type.")),
                }
                let file_size: usize = f.size() as usize;
                let mut buffer = vec![0; file_size];

                // read the file's bytes into a buffer
                match f.read_exact(&mut buffer) {
                    Err(e) => return Err(e),
                    Ok(()) => (),
                }
                buffer
            }
        };

        if buffer.len() < 375 {
            // The buffer is less than the header size. This is a sign
            // that there is something wrong with the file. Issue an error
            return Err(Error::new(ErrorKind::InvalidData,
                    format!("The file {} appears to be formatted incorrectly. Buffer size is smaller than the LAS header size.", self.get_short_filename())));
        }

        self.header.project_id_used = true;
        self.header.version_major = buffer[24];
        self.header.version_minor = buffer[25];

        if self.header.version_major < 1
            || self.header.version_major > 2
            || self.header.version_minor > 5
        {
            // There's something wrong. It could be that the project ID values, which are optional,
            // are not included in the header.
            self.header.version_major = buffer[8];
            self.header.version_minor = buffer[9];
            if self.header.version_major < 1
                || self.header.version_major > 2
                || self.header.version_minor > 5
            {
                // There's something very wrong. Throw an error.
                return Err(Error::new(ErrorKind::Other, format!("Error reading: {}\nIncorrect file version {}.{}\nEither the file is formatted incorrectly or it is an unsupported LAS version.", self.file_name, self.header.version_major, self.header.version_minor)));
            }
            self.header.project_id_used = false;
        }

        let mut bor =
            ByteOrderReader::<Cursor<Vec<u8>>>::new(Cursor::new(buffer), Endianness::LittleEndian);

        bor.seek(0);
        self.header.file_signature = bor.read_utf8(4);
        if self.header.file_signature != "LASF" {
            return Err(Error::new(ErrorKind::Other, format!("Error reading: {}\nIncorrect LAS file signature: {}.\nEither the file is formatted incorrectly or it is an unsupported LAS version.", self.file_name, self.header.file_signature)));
        }
        self.header.file_source_id = bor.read_u16()?;
        let ge_val = bor.read_u16()?;
        self.header.global_encoding = GlobalEncodingField { value: ge_val };
        if self.header.project_id_used {
            self.header.project_id1 = bor.read_u32()?;
            self.header.project_id2 = bor.read_u16()?;
            self.header.project_id3 = bor.read_u16()?;
            for i in 0..8 {
                self.header.project_id4[i] = bor.read_u8()?;
            }
        }
        // The version major and minor are read earlier.
        // Two bytes that must be added to the offset here.
        bor.inc_pos(2);
        self.header.system_id = bor.read_utf8(32);
        self.header.generating_software = bor.read_utf8(32);
        self.header.file_creation_day = bor.read_u16()?;
        self.header.file_creation_year = bor.read_u16()?;
        self.header.header_size = bor.read_u16()?;
        self.header.offset_to_points = bor.read_u32()?;
        self.header.number_of_vlrs = bor.read_u32()?;
        self.header.point_format = bor.read_u8()?;
        self.header.point_record_length = bor.read_u16()?;
        self.header.number_of_points_old = bor.read_u32()?;

        for i in 0..5 {
            self.header.number_of_points_by_return_old[i] = bor.read_u32()?;
        }
        self.header.x_scale_factor = bor.read_f64()?;
        self.header.y_scale_factor = bor.read_f64()?;
        self.header.z_scale_factor = bor.read_f64()?;
        self.header.x_offset = bor.read_f64()?;
        self.header.y_offset = bor.read_f64()?;
        self.header.z_offset = bor.read_f64()?;
        self.header.max_x = bor.read_f64()?;
        self.header.min_x = bor.read_f64()?;
        self.header.max_y = bor.read_f64()?;
        self.header.min_y = bor.read_f64()?;
        self.header.max_z = bor.read_f64()?;
        self.header.min_z = bor.read_f64()?;

        if self.header.version_major == 1 && self.header.version_minor >= 3 {
            self.header.waveform_data_start = bor.read_u64()?;
            if self.header.version_major == 1 && self.header.version_minor > 3 {
                self.header.offset_to_ex_vlrs = bor.read_u64()?;
                self.header.number_of_extended_vlrs = bor.read_u32()?;
                self.header.number_of_points = bor.read_u64()?;
                for i in 0..15 {
                    self.header.number_of_points_by_return[i] = bor.read_u64()?;
                }
            }
        }

        if self.header.number_of_points_old != 0 {
            self.header.number_of_points = self.header.number_of_points_old as u64;
            for i in 0..5 {
                if self.header.number_of_points_by_return_old[i] as u64
                    > self.header.number_of_points_by_return[i]
                {
                    self.header.number_of_points_by_return[i] =
                        self.header.number_of_points_by_return_old[i] as u64;
                }
            }
        } else if self.header.number_of_points_old == 0 && self.header.version_minor <= 3 {
            println!("Error reading the LAS file: The file does not appear to contain any points");
            self.header.number_of_points = 0;
        }

        // if self.file_mode != "rh" {
        // file_mode = "rh" does not read points or the VLR data, only the header.

        ///////////////////////
        // Read the VLR data //
        ///////////////////////
        bor.seek(self.header.header_size as usize);
        for _ in 0..self.header.number_of_vlrs {
            let mut vlr: Vlr = Default::default();
            vlr.reserved = bor.read_u16()?;
            vlr.user_id = bor.read_utf8(16);
            vlr.record_id = bor.read_u16()?;
            vlr.record_length_after_header = bor.read_u16()?;
            vlr.description = bor.read_utf8(32);
            // get the byte data
            for _ in 0..vlr.record_length_after_header {
                vlr.binary_data.push(bor.read_u8()?);
            }

            if vlr.record_id == 34_735 {
                self.geokeys
                    .add_key_directory(&vlr.binary_data, Endianness::LittleEndian);
            } else if vlr.record_id == 34_736 {
                self.geokeys
                    .add_double_params(&vlr.binary_data, Endianness::LittleEndian);
            } else if vlr.record_id == 34_737 {
                self.geokeys.add_ascii_params(&vlr.binary_data);
            } else if vlr.record_id == 2112 {
                let skip = if vlr.binary_data[vlr.binary_data.len() - 1] == 0u8 {
                    1
                } else {
                    0
                };
                self.wkt =
                    String::from_utf8_lossy(&vlr.binary_data[0..vlr.binary_data.len() - skip])
                        .trim()
                        .to_string();
            }
            self.vlr_data.push(vlr);
        }

        if self.file_mode != "rh" {
            // file_mode = "rh" does not read points, only the header and VLR data.

            /////////////////////////
            // Read the point data //
            /////////////////////////

            if self.header.number_of_points == 0 {
                return Ok(());
            }

            // Intensity and userdata are both optional. Figure out if they need to be read.
            // The only way to do this is to compare the point record length by point format
            let rec_lengths = [
                [20_u16, 18_u16, 19_u16, 17_u16],
                [28_u16, 26_u16, 27_u16, 25_u16],
                [26_u16, 24_u16, 25_u16, 23_u16],
                [34_u16, 32_u16, 33_u16, 31_u16],
                [57_u16, 55_u16, 56_u16, 54_u16],
                [63_u16, 61_u16, 62_u16, 60_u16],
                [30_u16, 28_u16, 29_u16, 27_u16],
                [36_u16, 34_u16, 35_u16, 33_u16],
                [38_u16, 36_u16, 37_u16, 35_u16],
                [59_u16, 57_u16, 58_u16, 56_u16],
                [67_u16, 65_u16, 66_u16, 64_u16],
            ];

            let mut skip_bytes = 0usize;

            if self.header.point_record_length == rec_lengths[self.header.point_format as usize][0]
            {
                self.use_point_intensity = true;
                self.use_point_userdata = true;
            } else if self.header.point_record_length
                == rec_lengths[self.header.point_format as usize][1]
            {
                self.use_point_intensity = false;
                self.use_point_userdata = true;
            } else if self.header.point_record_length
                == rec_lengths[self.header.point_format as usize][2]
            {
                self.use_point_intensity = true;
                self.use_point_userdata = false;
            } else if self.header.point_record_length
                == rec_lengths[self.header.point_format as usize][3]
            {
                self.use_point_intensity = false;
                self.use_point_userdata = false;
            } else if self.header.point_record_length
                > rec_lengths[self.header.point_format as usize][0]
            {
                // There must be some extra data in each point record. I've seen
                // this before with the output of LASTools. Assume the point intensity
                // and user data are both present.
                self.use_point_intensity = true;
                self.use_point_userdata = true;
                skip_bytes = (self.header.point_record_length
                    - rec_lengths[self.header.point_format as usize][0])
                    as usize;
            }

            self.point_data = Vec::with_capacity(self.header.number_of_points as usize);
            let mut p: PointData = Default::default();
            bor.seek(self.header.offset_to_points as usize);
            if self.header.point_format == 0 {
                for _ in 0..self.header.number_of_points {
                    // bor.seek(
                    //     self.header.offset_to_points as usize
                    //         + (i as usize) * (self.header.point_record_length as usize),
                    // );
                    // p = Default::default();
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.scan_angle = bor.read_i8()? as i16;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 1 {
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                for _ in 0..self.header.number_of_points {
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.scan_angle = bor.read_i8()? as i16;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 2 {
                self.colour_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut rgb: ColourData = Default::default();
                for _ in 0..self.header.number_of_points {
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.scan_angle = bor.read_i8()? as i16;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the RGB data
                    rgb.red = bor.read_u16()?;
                    rgb.green = bor.read_u16()?;
                    rgb.blue = bor.read_u16()?;
                    self.colour_data.push(rgb);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 3 {
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.colour_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut rgb: ColourData = Default::default();
                bor.seek(self.header.offset_to_points as usize);
                for _ in 0..self.header.number_of_points {
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.scan_angle = bor.read_i8()? as i16;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    // read the RGB data
                    rgb.red = bor.read_u16()?;
                    rgb.green = bor.read_u16()?;
                    rgb.blue = bor.read_u16()?;
                    self.colour_data.push(rgb);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 4 {
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.waveform_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut wfp: WaveformPacket;
                for _ in 0..self.header.number_of_points {
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.scan_angle = bor.read_i8()? as i16;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    // read the waveform data
                    wfp = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8()?;
                    wfp.offset_to_waveform_data = bor.read_u64()?;
                    wfp.waveform_packet_size = bor.read_u32()?;
                    wfp.ret_point_waveform_loc = bor.read_f32()?;
                    wfp.xt = bor.read_f32()?;
                    wfp.yt = bor.read_f32()?;
                    wfp.zt = bor.read_f32()?;
                    self.waveform_data.push(wfp);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 5 {
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.colour_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.waveform_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut rgb: ColourData = Default::default();
                let mut wfp: WaveformPacket;
                for _ in 0..self.header.number_of_points {
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.scan_angle = bor.read_i8()? as i16;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    // read the RGB data
                    rgb.red = bor.read_u16()?;
                    rgb.green = bor.read_u16()?;
                    rgb.blue = bor.read_u16()?;
                    self.colour_data.push(rgb);
                    // read the waveform data
                    wfp = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8()?;
                    wfp.offset_to_waveform_data = bor.read_u64()?;
                    wfp.waveform_packet_size = bor.read_u32()?;
                    wfp.ret_point_waveform_loc = bor.read_f32()?;
                    wfp.xt = bor.read_f32()?;
                    wfp.yt = bor.read_f32()?;
                    wfp.zt = bor.read_f32()?;
                    self.waveform_data.push(wfp);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 6 {
                // 64-bit
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                for _ in 0..self.header.number_of_points {
                    p.is_64bit = true;
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.classification = bor.read_u8()?;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.scan_angle = bor.read_i16()?;
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 7 {
                // 64-bit
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.colour_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut rgb: ColourData = Default::default();
                for _ in 0..self.header.number_of_points {
                    p.is_64bit = true;
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.classification = bor.read_u8()?;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.scan_angle = bor.read_i16()?;
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    // read the RGB data
                    rgb.red = bor.read_u16()?;
                    rgb.green = bor.read_u16()?;
                    rgb.blue = bor.read_u16()?;
                    self.colour_data.push(rgb);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 8 {
                // 64-bit
                // adds a NIR band to Point Format 7
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.colour_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut rgb: ColourData = Default::default();
                for _ in 0..self.header.number_of_points {
                    p.is_64bit = true;
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.classification = bor.read_u8()?;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.scan_angle = bor.read_i16()?;
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    // read the RGBNIR data
                    rgb.red = bor.read_u16()?;
                    rgb.green = bor.read_u16()?;
                    rgb.blue = bor.read_u16()?;
                    rgb.nir = bor.read_u16()?;
                    self.colour_data.push(rgb);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 9 {
                // 64-bit
                // adds waveform packets to Point Format 6
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.waveform_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut wfp: WaveformPacket;
                for _ in 0..self.header.number_of_points {
                    p.is_64bit = true;
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.classification = bor.read_u8()?;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.scan_angle = bor.read_i16()?;
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    // read the waveform data
                    wfp = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8()?;
                    wfp.offset_to_waveform_data = bor.read_u64()?;
                    wfp.waveform_packet_size = bor.read_u32()?;
                    wfp.ret_point_waveform_loc = bor.read_f32()?;
                    wfp.xt = bor.read_f32()?;
                    wfp.yt = bor.read_f32()?;
                    wfp.zt = bor.read_f32()?;
                    self.waveform_data.push(wfp);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            } else if self.header.point_format == 10 {
                // 64-bit
                // Everything in one record
                self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.colour_data = Vec::with_capacity(self.header.number_of_points as usize);
                self.waveform_data = Vec::with_capacity(self.header.number_of_points as usize);
                let mut rgb: ColourData = Default::default();
                let mut wfp: WaveformPacket;
                for _ in 0..self.header.number_of_points {
                    p.is_64bit = true;
                    p.x = bor.read_i32()?; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32()?; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32()?; // as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity {
                        p.intensity = bor.read_u16()?;
                    }
                    p.point_bit_field = bor.read_u8()?;
                    p.class_bit_field = bor.read_u8()?;
                    p.classification = bor.read_u8()?;
                    if self.use_point_userdata {
                        p.user_data = bor.read_u8()?;
                    }
                    p.scan_angle = bor.read_i16()?;
                    p.point_source_id = bor.read_u16()?;
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64()?);
                    // read the RGBNIR data
                    rgb.red = bor.read_u16()?;
                    rgb.green = bor.read_u16()?;
                    rgb.blue = bor.read_u16()?;
                    rgb.nir = bor.read_u16()?;
                    self.colour_data.push(rgb);
                    // read the waveform data
                    wfp = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8()?;
                    wfp.offset_to_waveform_data = bor.read_u64()?;
                    wfp.waveform_packet_size = bor.read_u32()?;
                    wfp.ret_point_waveform_loc = bor.read_f32()?;
                    wfp.xt = bor.read_f32()?;
                    wfp.yt = bor.read_f32()?;
                    wfp.zt = bor.read_f32()?;
                    self.waveform_data.push(wfp);
                    if skip_bytes > 0 {
                        bor.inc_pos(skip_bytes);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn read_laz_data(&mut self) -> Result<(), Error> {
        // At present, this uses the laz crate via the las-rs crate to convert into a WBT LasFile.
        // This doesn't seem like the most efficient way of doing this, and in the future I might
        // like to go via the crates directly.
        let mut reader = Reader::from_path(&self.file_name).expect("Error reading LAZ file.");
        let header = reader.header();
        let raw = header.clone().into_raw().unwrap();

        self.header.project_id_used = true;
        self.header.version_major = header.version().major;
        self.header.version_minor = header.version().minor;
 
        if self.header.version_major < 1
            || self.header.version_major > 2
            || self.header.version_minor > 5
        {
                // There's something very wrong. Throw an error.
                return Err(Error::new(ErrorKind::Other, format!("Error reading: {}\nIncorrect file version {}.{}\nEither the file is formatted incorrectly or it is an unsupported LAS version.", self.file_name, self.header.version_major, self.header.version_minor)));
        }

        self.header.file_signature = str::from_utf8(&raw.file_signature).unwrap().to_owned();
        self.header.file_source_id = header.file_source_id();
        self.header.global_encoding = GlobalEncodingField { value: raw.global_encoding };


        let guid = header.guid(); //.to_fields_le() -> (u32, u16, u16, &[u8; 8]);
        let fields = guid.to_fields_le();
        self.header.project_id1 = fields.0;
        self.header.project_id2 = fields.1; 
        self.header.project_id3 = fields.2;
        self.header.project_id4 = fields.3.clone();

        self.header.system_id = str::from_utf8(&raw.system_identifier.to_owned()).unwrap().to_owned();
        self.header.generating_software = header.generating_software().to_owned();
        self.header.file_creation_day = raw.file_creation_day_of_year;
        self.header.file_creation_year = raw.file_creation_year;
        self.header.header_size = raw.header_size;
        self.header.offset_to_points = raw.offset_to_point_data;
        self.header.number_of_vlrs = raw.number_of_variable_length_records;
        self.header.point_format = header.point_format().to_u8().unwrap(); // raw.point_data_record_format;
        self.header.point_record_length = raw.point_data_record_length;
        self.header.number_of_points_old = raw.number_of_point_records;
        self.header.number_of_points = raw.number_of_point_records as u64;
        self.header.number_of_points_by_return_old = raw.number_of_points_by_return.clone();

        let transforms = header.transforms().clone();
        self.header.x_scale_factor = transforms.x.scale;
        self.header.y_scale_factor = transforms.y.scale;
        self.header.z_scale_factor = transforms.z.scale;
        self.header.x_offset = transforms.x.offset;
        self.header.y_offset = transforms.y.offset;
        self.header.z_offset = transforms.z.offset;
        
        let bounds = header.bounds();
        self.header.max_x = bounds.max.x;
        self.header.min_x = bounds.min.x;
        self.header.max_y = bounds.max.y;
        self.header.min_y = bounds.min.y;
        self.header.max_z = bounds.max.z;
        self.header.min_z = bounds.min.z;

        /*
        if self.header.version_major == 1 && self.header.version_minor >= 3 {
            self.header.waveform_data_start = raw.start_of_waveform_data_packet_record;
            if self.header.version_major == 1 && self.header.version_minor > 3 {
                self.header.offset_to_ex_vlrs = bor.read_u64()?;
                self.header.number_of_extended_vlrs = bor.read_u32()?;
                self.header.number_of_points = bor.read_u64()?;
                for i in 0..15 {
                    self.header.number_of_points_by_return[i] = bor.read_u64()?;
                }
            }
        }

        if self.header.number_of_points_old != 0 {
            self.header.number_of_points = self.header.number_of_points_old as u64;
            for i in 0..5 {
                if self.header.number_of_points_by_return_old[i] as u64
                    > self.header.number_of_points_by_return[i]
                {
                    self.header.number_of_points_by_return[i] =
                        self.header.number_of_points_by_return_old[i] as u64;
                }
            }
        } else if self.header.number_of_points_old == 0 && self.header.version_minor <= 3 {
            println!("Error reading the LAS file: The file does not appear to contain any points");
            self.header.number_of_points = 0;
        }

         */

         ///////////////////////
        // Read the VLR data //
        ///////////////////////
        for v1 in header.vlrs() {
            let mut v = v1.clone();
            // The following is a bit of a hack. Either the las or laz crate has an error 
            // where it is reading a VLR description in more than 32 characters.
            while v.description.len() > 32 {
                v.description.pop();
            }
            let raw_vlr = v.clone().into_raw(false).unwrap();

            let mut vlr: Vlr = Default::default();
            vlr.reserved = raw_vlr.reserved;
            vlr.user_id = str::from_utf8(&raw_vlr.user_id.clone()).unwrap().to_owned();
            vlr.record_id = raw_vlr.record_id;
            vlr.record_length_after_header = match raw_vlr.record_length_after_header {
                RecordLength::Vlr(v) => v,
                RecordLength::Evlr(v) => v as u16,
            }; 
            
            let mut description = str::from_utf8(&raw_vlr.description.clone()).unwrap().to_owned();
            while description.len() > 32 {
                description.pop();
            }
            vlr.description = description;
            // get the byte data
            for j in 0..vlr.record_length_after_header {
                vlr.binary_data.push(raw_vlr.data[j as usize]);
            }

            if vlr.record_id == 34_735 {
                self.geokeys
                    .add_key_directory(&vlr.binary_data, Endianness::LittleEndian);
            } else if vlr.record_id == 34_736 {
                self.geokeys
                    .add_double_params(&vlr.binary_data, Endianness::LittleEndian);
            } else if vlr.record_id == 34_737 {
                self.geokeys.add_ascii_params(&vlr.binary_data);
            } else if vlr.record_id == 2112 {
                let skip = if vlr.binary_data[vlr.binary_data.len() - 1] == 0u8 {
                    1
                } else {
                    0
                };
                self.wkt =
                    String::from_utf8_lossy(&vlr.binary_data[0..vlr.binary_data.len() - skip])
                        .trim()
                        .to_string();
            }
            self.vlr_data.push(vlr);
        }


        if self.file_mode != "rh" {
            // Read the points into memory
            self.point_data = Vec::with_capacity(self.header.number_of_points as usize);
            self.gps_data = Vec::with_capacity(self.header.number_of_points as usize);
            self.colour_data = Vec::with_capacity(self.header.number_of_points as usize);
            self.waveform_data = Vec::with_capacity(self.header.number_of_points as usize);
            let mut rgb: ColourData;
            let mut wfp: WaveformPacket;
                    

            for wrapped_point in reader.points() {
                let point = wrapped_point.unwrap();
                let raw_point = point.into_raw(&transforms).unwrap();

                let mut p: PointData = Default::default();
                p.x = raw_point.x;
                p.y = raw_point.y;
                p.z = raw_point.z;

                // if self.use_point_intensity {
                    p.intensity = raw_point.intensity;
                // }
                let flags = raw_point.flags;
                p.set_return_number(flags.return_number());
                p.set_number_of_returns(flags.number_of_returns());
                p.set_classification(u8::from(flags.to_classification().unwrap()));
                p.set_scan_direction_flag(flags.scan_direction() == las::point::ScanDirection::LeftToRight);
                p.set_synthetic(flags.is_synthetic());
                p.set_keypoint(flags.is_key_point());
                p.set_withheld(flags.is_withheld());
                p.set_overlap(flags.is_overlap());
                p.set_scanner_channel(flags.scanner_channel());
                p.set_edge_of_flightline_flag(flags.is_edge_of_flight_line());


                // match flags {
                //     TwoByte(b1, b2) => {
                //         p.point_bit_field = b1;
                //         p.class_bit_field = b2;
                //     },
                //     ThreeByte(b1, b2, b3) => {
                //         p.point_bit_field = b1;
                //         p.class_bit_field = b2;
                //         p.classification = b3;
                //     },
                // }
                
                // if self.use_point_userdata {
                    p.user_data = raw_point.user_data;
                // }
                p.scan_angle = match raw_point.scan_angle {
                    ScanAngle::Rank(value) => value as i16,
                    ScanAngle::Scaled(value) => value,
                };
                p.point_source_id = raw_point.point_source_id;
                self.point_data.push(p);

                if raw_point.gps_time.is_some() {
                    self.gps_data.push(raw_point.gps_time.unwrap());
                }
                

                // read the RGB/NIR data
                if raw_point.color.is_some() {
                    let colour = raw_point.color.unwrap();

                    rgb = Default::default();
                    rgb.red = colour.red;
                    rgb.green = colour.green;
                    rgb.blue = colour.blue;

                    if raw_point.nir.is_some() {
                        rgb.nir = raw_point.nir.unwrap();
                    }

                    self.colour_data.push(rgb);
                }
                
                // read the waveform data
                if raw_point.waveform.is_some() {
                    let waveform = raw_point.waveform.unwrap();
                    wfp = Default::default();
                    wfp.packet_descriptor_index = waveform.wave_packet_descriptor_index;
                    wfp.offset_to_waveform_data = waveform.byte_offset_to_waveform_data;
                    wfp.waveform_packet_size = waveform.waveform_packet_size_in_bytes;
                    wfp.ret_point_waveform_loc = waveform.return_point_waveform_location;
                    wfp.xt = waveform.x_t;
                    wfp.yt = waveform.y_t;
                    wfp.zt = waveform.z_t;
                    self.waveform_data.push(wfp);
                }
            }

        }

        drop(raw);
        // drop(header);
        drop(reader);

        Ok(())

    }

    pub fn read_zlidar_data(&mut self) -> Result<(), Error> {
        let mut f = File::open(&self.file_name).expect("Error opening LAS file.");
        let metadata = fs::metadata(&self.file_name)?;
        let file_size: usize = metadata.len() as usize;

        let mut buffer = vec![0; file_size];
        if file_size < 1024 * 1024 * 500 {
            // 2147483646 is the actual maximum file read on Mac
            f.read(&mut buffer)?;
        } else {
            let block_size = 1024 * 1024 * 500;
            let mut start_byte = 0usize;
            let mut end_byte = block_size;
            let mut bytes_read = 0;
            while bytes_read < file_size {
                f.read(&mut buffer[start_byte..end_byte])?;
                start_byte += block_size;
                end_byte += block_size;
                if end_byte > file_size {
                    end_byte = file_size;
                }
                bytes_read += block_size;
            }
        }

        if buffer.len() < 375 {
            // The buffer is less than the header size. This is a sign
            // that there is something wrong with the file. Issue an error
            return Err(Error::new(ErrorKind::InvalidData,
                    format!("The file {} appears to be formatted incorrectly. Buffer size is smaller than the LAS header size.", self.get_short_filename())));
        }

        self.header.project_id_used = true;
        self.header.version_major = buffer[24];
        self.header.version_minor = buffer[25];

        if self.header.version_major < 1
            || self.header.version_major > 2
            || self.header.version_minor > 5
        {
            // There's something wrong. It could be that the project ID values, which are optional,
            // are not included in the header.
            self.header.version_major = buffer[8];
            self.header.version_minor = buffer[9];
            if self.header.version_major < 1
                || self.header.version_major > 2
                || self.header.version_minor > 5
            {
                // There's something very wrong. Throw an error.
                return Err(Error::new(ErrorKind::Other, format!("Error reading: {}\nIncorrect file version {}.{}\nEither the file is formatted incorrectly or it is an unsupported LAS version.", self.file_name, self.header.version_major, self.header.version_minor)));
            }
            self.header.project_id_used = false;
        }

        let mut bor =
            ByteOrderReader::<Cursor<Vec<u8>>>::new(Cursor::new(buffer), Endianness::LittleEndian);

        bor.seek(0);
        self.header.file_signature = bor.read_utf8(4);
        if self.header.file_signature != "ZLDR" {
            return Err(Error::new(ErrorKind::Other, format!("Error reading: {}\nIncorrect zlidar file signature: {}.\nEither the file is formatted incorrectly or it is an unsupported zlidar version.", self.file_name, self.header.file_signature)));
        }
        self.header.file_source_id = bor.read_u16()?;
        let ge_val = bor.read_u16()?;
        self.header.global_encoding = GlobalEncodingField { value: ge_val };
        if self.header.project_id_used {
            self.header.project_id1 = bor.read_u32()?;
            self.header.project_id2 = bor.read_u16()?;
            self.header.project_id3 = bor.read_u16()?;
            for i in 0..8 {
                self.header.project_id4[i] = bor.read_u8()?;
            }
        }
        // The version major and minor are read earlier.
        // Two bytes that must be added to the offset here.
        bor.inc_pos(2);
        self.header.system_id = bor.read_utf8(32);
        self.header.generating_software = bor.read_utf8(32);
        self.header.file_creation_day = bor.read_u16()?;
        self.header.file_creation_year = bor.read_u16()?;
        self.header.header_size = bor.read_u16()?;
        self.header.offset_to_points = bor.read_u32()?;
        self.header.number_of_vlrs = bor.read_u32()?;
        self.header.point_format = bor.read_u8()?;
        self.header.point_record_length = bor.read_u16()?;
        self.header.number_of_points_old = bor.read_u32()?;

        for i in 0..5 {
            self.header.number_of_points_by_return_old[i] = bor.read_u32()?;
        }
        self.header.x_scale_factor = bor.read_f64()?;
        self.header.y_scale_factor = bor.read_f64()?;
        self.header.z_scale_factor = bor.read_f64()?;
        self.header.x_offset = bor.read_f64()?;
        self.header.y_offset = bor.read_f64()?;
        self.header.z_offset = bor.read_f64()?;
        self.header.max_x = bor.read_f64()?;
        self.header.min_x = bor.read_f64()?;
        self.header.max_y = bor.read_f64()?;
        self.header.min_y = bor.read_f64()?;
        self.header.max_z = bor.read_f64()?;
        self.header.min_z = bor.read_f64()?;

        if self.header.version_major == 1 && self.header.version_minor >= 3 {
            self.header.waveform_data_start = bor.read_u64()?;
            if self.header.version_major == 1 && self.header.version_minor > 3 {
                self.header.offset_to_ex_vlrs = bor.read_u64()?;
                self.header.number_of_extended_vlrs = bor.read_u32()?;
                self.header.number_of_points = bor.read_u64()?;
                for i in 0..15 {
                    self.header.number_of_points_by_return[i] = bor.read_u64()?;
                }
            }
        }

        if self.header.number_of_points_old != 0 {
            self.header.number_of_points = self.header.number_of_points_old as u64;
            for i in 0..5 {
                if self.header.number_of_points_by_return_old[i] as u64
                    > self.header.number_of_points_by_return[i]
                {
                    self.header.number_of_points_by_return[i] =
                        self.header.number_of_points_by_return_old[i] as u64;
                }
            }
        } else if self.header.number_of_points_old == 0 && self.header.version_minor <= 3 {
            println!("Error reading the LAS file: The file does not appear to contain any points");
            self.header.number_of_points = 0;
        }

        ///////////////////////
        // Read the VLR data //
        ///////////////////////
        bor.seek(self.header.header_size as usize);
        for _ in 0..self.header.number_of_vlrs {
            let mut vlr: Vlr = Default::default();
            vlr.reserved = bor.read_u16()?;
            vlr.user_id = bor.read_utf8(16);
            vlr.record_id = bor.read_u16()?;
            vlr.record_length_after_header = bor.read_u16()?;
            vlr.description = bor.read_utf8(32);
            // get the byte data
            for _ in 0..vlr.record_length_after_header {
                vlr.binary_data.push(bor.read_u8()?);
            }

            if vlr.record_id == 34_735 {
                self.geokeys
                    .add_key_directory(&vlr.binary_data, Endianness::LittleEndian);
            } else if vlr.record_id == 34_736 {
                self.geokeys
                    .add_double_params(&vlr.binary_data, Endianness::LittleEndian);
            } else if vlr.record_id == 34_737 {
                self.geokeys.add_ascii_params(&vlr.binary_data);
            } else if vlr.record_id == 2112 {
                let skip = if vlr.binary_data[vlr.binary_data.len() - 1] == 0u8 {
                    1
                } else {
                    0
                };
                self.wkt =
                    String::from_utf8_lossy(&vlr.binary_data[0..vlr.binary_data.len() - skip])
                        .trim()
                        .to_string();
            }
            self.vlr_data.push(vlr);
        }

        if self.file_mode != "rh" {
            // file_mode = "rh" does not read points, only the header and VLR data.

            /////////////////////////
            // Read the point data //
            /////////////////////////

            if self.header.number_of_points == 0 {
                return Ok(());
            }

            self.point_data = vec![Default::default(); self.header.number_of_points as usize];

            if self.header.point_format == 2
                || self.header.point_format == 3
                || self.header.point_format == 5
                || self.header.point_format == 7
                || self.header.point_format == 8
            {
                self.colour_data = vec![Default::default(); self.header.number_of_points as usize];
            }

            let mut next_offset = self.header.offset_to_points as usize;
            let mut point_num = 0;
            let mut block_bytes: u64;
            let mut pt: usize;
            let mut num_points_in_block = 0usize;
            let mut flag = true;
            let mut num_fields: u8;
            let mut compression_method: u8;
            let (mut major_version, mut minor_version): (u8, u8);

            // read the next four bytes to determine what the zLidar version
            bor.seek(next_offset);
            num_fields = bor.read_u8().expect("Error while reading byte data.");
            let compression_byte = bor.read_u8().expect("Error while reading byte data.");
            
            compression_method =  compression_byte & 0b0000_0111;
            let compression_level = (compression_byte & 0b1111_1000) >> 3;
            
            // if compression_method != 0 && compression_method != 1 {
            //     return Err(Error::new(
            //         ErrorKind::Other,
            //         "Unsupported compression method.",
            //     ));
            // }

            self.compression = match compression_method {
                0 => { ZlidarCompression::Deflate { level: compression_level } },
                1 => { ZlidarCompression::Brotli { level: compression_level } },
                _ => {return Err(Error::new(
                    ErrorKind::Other,
                    "Unsupported compression method.",
                ));}
            };

            major_version = bor.read_u8().expect("Error while reading byte data.");
            minor_version = bor.read_u8().expect("Error while reading byte data.");
            // println!("num_fields: {} compression_method: {} major_version: {} minor_version: {}", num_fields, compression_method, major_version, minor_version);
            if major_version == 1 && minor_version == 1 {
                let mut field_code: u8;
                let mut offset: u64;
                let mut num_bytes: u64;
                let mut change_bytes: Vec<u8> = vec![];
                let mut change_byte_read: bool;
                let mut scanner_chan_read: bool;
                let mut ret_num_read: bool;
                let mut num_rets_read: bool;
                let mut val_u8: u8;
                let mut val_num: usize;
                let mut scan_chan: usize;
                let mut cntx: usize;

                next_offset = self.header.offset_to_points as usize + 4;
                while flag {
                    bor.seek(next_offset);
                    // println!("offset: {}", next_offset);
                    block_bytes = 0;
                    change_byte_read = false;
                    scanner_chan_read = false;
                    ret_num_read = false;
                    num_rets_read = false;

                    for _ in 0..num_fields {
                        // Read the field header
                        field_code = bor.read_u8().expect("Error while reading byte data.");
                        offset = bor.read_u64().expect("Error while reading byte data.");
                        num_bytes = bor.read_u64().expect("Error while reading byte data.");
                        block_bytes += 17 + num_bytes;
                        
                        // println!("field_code: {} offset: {} num_bytes: {} block_bytes: {}", field_code, offset, num_bytes, block_bytes);
                        
                        // Decompress the bytes
                        bor.seek(offset as usize);
                        let mut compressed = vec![0u8; num_bytes as usize];
                        bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                        let decompressed = if compression_method == 0 { 
                            // DEFLATE
                            decompress_to_vec_zlib(&compressed).expect("DEFLATE failed to decompress data.")
                        } else if compression_method == 1 {
                            // brotli
                            brotli_decompress(&compressed)
                        } else {
                            panic!("Unrecognized compression method.")
                        };

                        match field_code {
                            0 => { // Change byte
                                // println!("field_code: {} offset: {} num_bytes: {} {:?}", field_code, offset, num_bytes, compressed);
                                change_bytes = decompressed.clone();
                                num_points_in_block = change_bytes.len();
                                change_byte_read = true;
                            }, 
                            1 => { // Scanner channel
                                if !change_byte_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                // Convert to values
                                val_num = 0usize;
                                val_u8 = decompressed[val_num];
                                let mut scan_chan = val_u8 & 0b0000_0011u8;
                                let mut prev_val = scan_chan;
                                self.point_data[point_num].set_scanner_channel(scan_chan);
                                let num_bits = 2;
                                let mut num_bits_read = num_bits;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    if (change_bytes[j] & 0b0000_0001u8) == 1u8 {
                                        scan_chan = (val_u8 >> num_bits_read) & 0b0000_0011u8;
                                        self.point_data[pt].set_scanner_channel(scan_chan);
                                        num_bits_read += num_bits;
                                        if num_bits_read == 8 {
                                            val_num += 1;
                                            val_u8 = decompressed[val_num];
                                            num_bits_read = 0;
                                        }
                                        prev_val = scan_chan;
                                    } else {
                                        self.point_data[pt].set_scanner_channel(prev_val);
                                    }
                                }

                                scanner_chan_read = true;
                            },
                            2 => { // Return number
                                if !change_byte_read || !scanner_chan_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                // Convert to values
                                val_num = 0usize;
                                val_u8 = decompressed[val_num];
                                let mut ret_num = val_u8 & 0b0000_1111u8;
                                self.point_data[point_num].set_return_number(ret_num);
                                let num_bits = 4;
                                let mut num_bits_read = num_bits;
                                let mut prev_vals = [ret_num, ret_num, ret_num, ret_num];
                                let mut ret_num_diff: u8;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    ret_num_diff = (change_bytes[j] & 0b0000_1100u8) >> 2;
                                    if ret_num_diff == 0 {
                                        // same as previous for scan chan
                                        self.point_data[pt].set_return_number(prev_vals[scan_chan]);
                                    } else if ret_num_diff == 1 {
                                        // one more than previous for scan chan
                                        self.point_data[pt].set_return_number(prev_vals[scan_chan] + 1);
                                        prev_vals[scan_chan] += 1;
                                    } else if ret_num_diff == 2 {
                                        // one less than previous for scan chan
                                        self.point_data[pt].set_return_number(prev_vals[scan_chan] - 1);
                                        prev_vals[scan_chan] -= 1;
                                    } else { // 3
                                        // new value stored in data
                                        ret_num = (val_u8 >> num_bits_read) & 0b0000_1111u8;
                                        self.point_data[pt].set_return_number(ret_num);
                                        num_bits_read += num_bits;
                                        if num_bits_read == 8 && val_num < decompressed.len()-1 {
                                            val_num += 1;
                                            val_u8 = decompressed[val_num];
                                            num_bits_read = 0;
                                        }
                                        prev_vals[scan_chan] = ret_num;
                                    }
                                }

                                ret_num_read = true;
                            }, 

                            3 => { // Number of returns
                                if !change_byte_read || !scanner_chan_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                // Convert to values
                                val_num = 0usize;
                                val_u8 = decompressed[val_num];
                                let mut num_rets = val_u8 & 0b0000_1111u8;
                                // println!("{} {}", point_num, num_rets);
                                self.point_data[point_num].set_number_of_returns(num_rets);
                                let num_bits = 4;
                                let mut num_bits_read = num_bits;
                                let mut prev_vals = [num_rets, num_rets, num_rets, num_rets];
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    if ((change_bytes[j] & 0b0001_0000u8) >> 4) == 1 {
                                        // new value stored in data
                                        num_rets = (val_u8 >> num_bits_read) & 0b0000_1111u8;
                                        self.point_data[pt].set_number_of_returns(num_rets);
                                        num_bits_read += num_bits;
                                        if num_bits_read == 8 && val_num < decompressed.len()-1 {
                                            val_num += 1;
                                            val_u8 = decompressed[val_num];
                                            num_bits_read = 0;
                                        }
                                        prev_vals[scan_chan] = num_rets;
                                    } else {
                                        self.point_data[pt].set_number_of_returns(prev_vals[scan_chan]);
                                    }

                                    // if pt >= 100_000 && pt < 100_100 {
                                    // // if pt >= 0 && pt < 100 {
                                    //     println!("{}, {}/{}", pt, self.point_data[pt].return_number(), self.point_data[pt].number_of_returns());
                                    // }
                                }

                                num_rets_read = true;
                            }, 

                            4 => { // X
                                if !change_byte_read || !scanner_chan_read || !ret_num_read || !num_rets_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                offset = bor.read_u64().expect("Error while reading byte data.");
                                num_bytes = bor.read_u64().expect("Error while reading byte data.");
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                block_bytes += 16 + num_bytes;
                                
                                // Decompress the bytes
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                let decompressed2 = if compression_method == 0 { 
                                    // DEFLATE
                                    decompress_to_vec_zlib(&compressed).expect("DEFLATE failed to decompress data.")
                                } else if compression_method == 1 {
                                    // brotli
                                    brotli_decompress(&compressed)
                                } else {
                                    panic!("Unrecognized compression method.")
                                };

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed2),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut delta_values = Vec::with_capacity(num_points_in_block);
                                let mut val2 = Vec::with_capacity(num_points_in_block);
                                val_num = 0usize;
                                val_u8 = decompressed[val_num];
                                let mut tag = val_u8 & 0b0000_1111u8;
                                let mut val_i32 = if tag == 15u8 {
                                    bor2.read_i32().expect("Error reading byte data.")
                                } else {
                                    panic!("Error reading coordinate data from zLidar file.");
                                };
                                val2.push(val_i32);
                                delta_values.push(val_i32);
                                let mut prev_vals = [val_i32, val_i32, val_i32, val_i32];
                                self.point_data[point_num].x = val_i32; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                                let num_bits = 4;
                                let mut num_bits_read = num_bits;
                                let mut prev_index = [
                                    [0; 16],
                                    [0; 16],
                                    [0; 16],
                                    [0; 16],
                                ];
                                for _ in 1..num_points_in_block {
                                    // pt = point_num + j;
                                    
                                    tag = (val_u8 >> num_bits_read) & 0b0000_1111u8;
                                    num_bits_read += num_bits;
                                    if num_bits_read == 8 && val_num < decompressed.len()-1 {
                                        val_num += 1;
                                        val_u8 = decompressed[val_num];
                                        num_bits_read = 0;
                                    }

                                    if tag < 13 {
                                        // the offset from prev_val is tag - 6
                                        val_i32 = tag as i32 - 6;
                                    } else if tag == 13 {
                                        // the offset is one byte
                                        val_i32 = bor2.read_i8().expect("Error reading byte data.") as i32;
                                    } else if tag == 14 {
                                        // the offset is two bytes
                                        val_i32 = bor2.read_i16().expect("Error reading byte data.") as i32;
                                    } else { // tag == 15
                                        // the offset is four bytes
                                        val_i32 = bor2.read_i32().expect("Error reading byte data.");
                                    }
                                    val2.push(val_i32);
                                }

                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    
                                    cntx = self.get_context(pt);
                                    let delta_j: i32 = val2[j] + delta_values[prev_index[scan_chan][cntx]];
                                    delta_values.push(delta_j);

                                    let val: i32 = prev_vals[scan_chan] + delta_j;

                                    self.point_data[pt].x = val; // as f64 * self.header.x_scale_factor + self.header.x_offset;
                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     println!("{}, {}/{}, {}, {}, {}", pt, self.point_data[pt].return_number(), self.point_data[pt].number_of_returns(), val2[j], delta_j, self.point_data[pt].x);
                                    // }
                                    prev_vals[scan_chan] = val;
                                    prev_index[scan_chan][cntx] = j;
                                }
                            },

                            5 => { // Y
                                if !change_byte_read || !scanner_chan_read || !ret_num_read || !num_rets_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                offset = bor.read_u64().expect("Error while reading byte data.");
                                num_bytes = bor.read_u64().expect("Error while reading byte data.");
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                block_bytes += 16 + num_bytes;
                                
                                // Decompress the bytes
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                let decompressed2 = if compression_method == 0 { 
                                    // DEFLATE
                                    decompress_to_vec_zlib(&compressed).expect("DEFLATE failed to decompress data.")
                                } else if compression_method == 1 {
                                    // brotli
                                    brotli_decompress(&compressed)
                                } else {
                                    panic!("Unrecognized compression method.")
                                };

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed2),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut delta_values = Vec::with_capacity(num_points_in_block);
                                let mut val2 = Vec::with_capacity(num_points_in_block);
                                val_num = 0usize;
                                val_u8 = decompressed[val_num];
                                let mut tag = val_u8 & 0b0000_1111u8;
                                let mut val_i32 = if tag == 15u8 {
                                    bor2.read_i32().expect("Error reading byte data.")
                                } else {
                                    panic!("Error reading coordinate data from zLidar file.");
                                };
                                val2.push(val_i32);
                                delta_values.push(val_i32);
                                let mut prev_vals = [val_i32, val_i32, val_i32, val_i32];
                                self.point_data[point_num].y = val_i32; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                                let num_bits = 4;
                                let mut num_bits_read = num_bits;
                                let mut prev_index = [
                                    [0; 16],
                                    [0; 16],
                                    [0; 16],
                                    [0; 16],
                                ];
                                for _ in 1..num_points_in_block {
                                    // pt = point_num + j;
                                    
                                    tag = (val_u8 >> num_bits_read) & 0b0000_1111u8;
                                    num_bits_read += num_bits;
                                    if num_bits_read == 8 && val_num < decompressed.len()-1 {
                                        val_num += 1;
                                        val_u8 = decompressed[val_num];
                                        num_bits_read = 0;
                                    }

                                    if tag < 13 {
                                        // the offset from prev_val is tag - 6
                                        val_i32 = tag as i32 - 6;
                                    } else if tag == 13 {
                                        // the offset is one byte
                                        val_i32 = bor2.read_i8().expect("Error reading byte data.") as i32;
                                    } else if tag == 14 {
                                        // the offset is two bytes
                                        val_i32 = bor2.read_i16().expect("Error reading byte data.") as i32;
                                    } else { // tag == 15
                                        // the offset is four bytes
                                        val_i32 = bor2.read_i32().expect("Error reading byte data.");
                                    }
                                    val2.push(val_i32);
                                }

                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    
                                    cntx = self.get_context(pt);
                                    let delta_j: i32 = val2[j] + delta_values[prev_index[scan_chan][cntx]];
                                    delta_values.push(delta_j);

                                    let val: i32 = prev_vals[scan_chan] + delta_j;

                                    self.point_data[pt].y = val; // as f64 * self.header.y_scale_factor + self.header.y_offset;
                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     println!("{}, {}, {}", pt, self.point_data[pt].x, self.point_data[pt].y);
                                    // }
                                    prev_vals[scan_chan] = val;
                                    prev_index[scan_chan][cntx] = j;
                                }
                            },

                            6 => { // Z
                                if !change_byte_read || !scanner_chan_read || !ret_num_read || !num_rets_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                offset = bor.read_u64().expect("Error while reading byte data.");
                                num_bytes = bor.read_u64().expect("Error while reading byte data.");
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                block_bytes += 16 + num_bytes;
                                
                                // Decompress the bytes
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                let decompressed2 = if compression_method == 0 { 
                                    // DEFLATE
                                    decompress_to_vec_zlib(&compressed).expect("DEFLATE failed to decompress data.")
                                } else if compression_method == 1 {
                                    // brotli
                                    brotli_decompress(&compressed)
                                } else {
                                    panic!("Unrecognized compression method.")
                                };

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed2),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut tag: u8;
                                let mut val_i32: i32;
                                let num_bits = 4;
                                val_num = 0usize;
                                let mut val_u8 = decompressed[val_num];
                                let mut num_bits_read = 0;
                                let mut prev_late_vals = [0i32, 0i32, 0i32, 0i32];
                                let mut prev_early_vals = [0i32, 0i32, 0i32, 0i32];
                                let mut prev_val = 0i32;
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;

                                    tag = (val_u8 >> num_bits_read) & 0b0000_1111u8;
                                    num_bits_read += num_bits;
                                    if num_bits_read == 8 && val_num < decompressed.len()-1 {
                                        val_num += 1;
                                        val_u8 = decompressed[val_num];
                                        num_bits_read = 0;
                                    }

                                    val_i32 = if tag < 13 {
                                        // the offset from prev_val is tag - 6
                                        tag as i32 - 6
                                    } else if tag == 13 {
                                        // the offset is one byte
                                        bor2.read_i8().expect("Error reading byte data.") as i32
                                    } else if tag == 14 {
                                        // the offset is two bytes
                                        bor2.read_i16().expect("Error reading byte data.") as i32
                                    } else { // tag == 15
                                        // the offset is four bytes
                                        bor2.read_i32().expect("Error reading byte data.")
                                    };

                                    prev_val = if self.point_data[pt].is_late_return() {
                                        prev_late_vals[scan_chan]
                                    } else {
                                        prev_early_vals[scan_chan]
                                    };

                                    val_i32 += prev_val;
                                    self.point_data[pt].z = val_i32; // as f64 * self.header.z_scale_factor + self.header.z_offset;

                                    if self.point_data[pt].is_late_return() {
                                        prev_late_vals[scan_chan] = val_i32;
                                    } else {
                                        prev_early_vals[scan_chan] = val_i32;
                                    }

                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     println!("{}, {}, {}, {}", pt, self.point_data[pt].x, self.point_data[pt].y, self.point_data[pt].z);
                                    // }
                                }
                            },

                            7 => { // Intensity
                                if !change_byte_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                self.point_data[point_num].intensity = bor2.read_u16().expect("Error while reading byte data.") as u16;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    if ((change_bytes[j] & 0b1000_0000u8) >> 7) == 1 { // 2 bytes
                                        self.point_data[pt].intensity = bor2.read_u16().expect("Error while reading byte data.");
                                    } else { // 1 byte
                                        self.point_data[pt].intensity = bor2.read_u8().expect("Error while reading byte data.") as u16;
                                    }

                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     let t = ((change_bytes[j] & 0b1000_0000u8) >> 7) == 1;
                                    //     println!("{}, {}, {}", pt, t, self.point_data[pt].intensity);
                                    // }
                                }
                            }, 

                            8 => { // Flags
                                if !change_byte_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                // Convert to values
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    val_u8 = decompressed[j];
                                    if val_u8 & 0b0000_0001u8 == 1 {
                                        self.point_data[pt].set_synthetic(true);
                                    } else {
                                        self.point_data[pt].set_synthetic(false);
                                    }

                                    if ((val_u8 & 0b0000_0010u8) >> 1) == 1 {
                                        self.point_data[pt].set_keypoint(true);
                                    } else {
                                        self.point_data[pt].set_keypoint(false);
                                    }

                                    if ((val_u8 & 0b0000_0100u8) >> 2) == 1 {
                                        self.point_data[pt].set_withheld(true);
                                    } else {
                                        self.point_data[pt].set_withheld(false);
                                    }

                                    if ((val_u8 & 0b0000_1000u8) >> 3) == 1 {
                                        self.point_data[pt].set_overlap(true);
                                    } else {
                                        self.point_data[pt].set_overlap(false);
                                    }

                                    if ((val_u8 & 0b0001_0000u8) >> 4) == 1 {
                                        self.point_data[pt].set_scan_direction_flag(true);
                                    } else {
                                        self.point_data[pt].set_scan_direction_flag(false);
                                    }

                                    if ((val_u8 & 0b0010_0000u8) >> 5) == 1 {
                                        self.point_data[pt].set_edge_of_flightline_flag(true);
                                    } else {
                                        self.point_data[pt].set_edge_of_flightline_flag(false);
                                    }
                                }
                            }, 

                            9 => { // Classification byte
                                if !change_byte_read || !scanner_chan_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                // Convert to values
                                self.point_data[point_num].set_classification(decompressed[0]);
                                let mut prev_val = [decompressed[0], decompressed[0], decompressed[0], decompressed[0]];
                                val_num = 0usize;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    if ((change_bytes[j] & 0b0010_0000u8) >> 5) == 1 {
                                        val_num += 1;
                                        self.point_data[pt].set_classification(decompressed[val_num]);
                                        prev_val[scan_chan] = decompressed[val_num];
                                    } else {
                                        self.point_data[pt].set_classification(prev_val[scan_chan]);
                                    }
                                }
                            }, 

                            10 => { // User data
                                if !change_byte_read || !scanner_chan_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                offset = bor.read_u64().expect("Error while reading byte data.");
                                num_bytes = bor.read_u64().expect("Error while reading byte data.");
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                block_bytes += 16 + num_bytes;
                                
                                // Decompress the bytes
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                let decompressed2 = if compression_method == 0 { 
                                    // DEFLATE
                                    decompress_to_vec_zlib(&compressed).expect("DEFLATE failed to decompress data.")
                                } else if compression_method == 1 {
                                    // brotli
                                    brotli_decompress(&compressed)
                                } else {
                                    panic!("Unrecognized compression method.")
                                };

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed2),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut tag: u8;
                                let val = bor2.read_u8().expect("Error while reading byte data.");
                                self.point_data[point_num].user_data = val;
                                let mut prev_val = [val, val, val, val];
                                val_num = 0usize;
                                val_u8 = decompressed[val_num];
                                let num_bits = 1;
                                let mut num_bits_read = num_bits;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    
                                    tag = (val_u8 >> num_bits_read) & 0b0000_0001u8;
                                    num_bits_read += num_bits;
                                    if num_bits_read == 8 && val_num < decompressed.len()-1 {
                                        val_num += 1;
                                        val_u8 = decompressed[val_num];
                                        num_bits_read = 0;
                                    }

                                    if tag == 1 {
                                        self.point_data[pt].user_data = bor2.read_u8().expect("Error while reading byte data.");
                                        prev_val[scan_chan] = self.point_data[pt].user_data;
                                    } else {
                                        self.point_data[pt].user_data = prev_val[scan_chan];
                                    }

                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     println!("{}, {}", pt, self.point_data[pt].user_data);
                                    // }
                                }
                            },

                            11 => { // Scan angle
                                if !change_byte_read || !scanner_chan_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut val = bor2.read_i16().expect("Error while reading byte data.");
                                self.point_data[point_num].scan_angle = val;
                                let mut prev_val = [val, val, val, val];
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    if ((change_bytes[j] & 0b0100_0000u8) >> 6) == 1 {
                                        val = bor2.read_i16().expect("Error while reading byte data.");
                                        self.point_data[pt].scan_angle = val;
                                        prev_val[scan_chan] = val;
                                    } else {
                                        self.point_data[pt].scan_angle = prev_val[scan_chan];
                                    }

                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     println!("{}, {}", pt, self.point_data[pt].scan_angle);
                                    // }
                                }
                            }, 

                            12 => { // PointSourceID
                                if !change_byte_read || !scanner_chan_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                offset = bor.read_u64().expect("Error while reading byte data.");
                                num_bytes = bor.read_u64().expect("Error while reading byte data.");
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                block_bytes += 16 + num_bytes;
                                
                                // Decompress the bytes
                                bor.seek(offset as usize);
                                let mut compressed = vec![0u8; num_bytes as usize];
                                bor.read_exact(&mut compressed).expect("Error while reading byte data.");
                                let decompressed2 = if compression_method == 0 { 
                                    // DEFLATE
                                    decompress_to_vec_zlib(&compressed).expect("DEFLATE failed to decompress data.")
                                } else if compression_method == 1 {
                                    // brotli
                                    brotli_decompress(&compressed)
                                } else {
                                    panic!("Unrecognized compression method.")
                                };

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed2),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut tag: u8;
                                let val = bor2.read_u16().expect("Error while reading byte data.");
                                self.point_data[point_num].point_source_id = val;
                                let mut prev_val = [val, val, val, val];
                                val_num = 0usize;
                                val_u8 = decompressed[val_num];
                                let num_bits = 1;
                                let mut num_bits_read = num_bits;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    
                                    tag = (val_u8 >> num_bits_read) & 0b0000_0001u8;
                                    num_bits_read += num_bits;
                                    if num_bits_read == 8 && val_num < decompressed.len()-1 {
                                        val_num += 1;
                                        val_u8 = decompressed[val_num];
                                        num_bits_read = 0;
                                    }

                                    if tag == 1 {
                                        self.point_data[pt].point_source_id = bor2.read_u16().expect("Error while reading byte data.");
                                        prev_val[scan_chan] = self.point_data[pt].point_source_id;
                                    } else {
                                        self.point_data[pt].point_source_id = prev_val[scan_chan];
                                    }

                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     println!("{}, {}", pt, self.point_data[pt].point_source_id);
                                    // }
                                }
                            },

                            13 => { // GPS time
                                if !change_byte_read || !scanner_chan_read || num_points_in_block == 0 {
                                    panic!("Point block fields do not appear to be in the proper order. The file will not be read.");
                                }

                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut val = bor2.read_f64().expect("Error while reading byte data.");
                                self.gps_data.push(val);
                                let mut prev_val = [val, val, val, val];
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    scan_chan = self.point_data[pt].scanner_channel() as usize;
                                    if ((change_bytes[j] & 0b0000_0010u8) >> 1) == 1 {
                                        val = bor2.read_f64().expect("Error while reading byte data.");
                                        self.gps_data.push(val + prev_val[scan_chan]);
                                        prev_val[scan_chan] = val + prev_val[scan_chan];
                                    } else {
                                        self.gps_data.push(prev_val[scan_chan]);
                                    }

                                    // if pt >= 100_000 && pt < 100_100 {
                                    //     println!("{}, {}", pt, self.gps_data[pt]);
                                    // }
                                }
                            }, 

                            14 => { // Red
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut val: u16;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    val = bor2.read_u16().expect("Error while reading byte data.");
                                    self.colour_data[pt].red = val;
                                }
                            },
                            
                            15 => { // Green
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut val: u16;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    val = bor2.read_u16().expect("Error while reading byte data.");
                                    self.colour_data[pt].green = val;
                                }
                            }, 

                            16 => { // Blue
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut val: u16;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    val = bor2.read_u16().expect("Error while reading byte data.");
                                    self.colour_data[pt].blue = val;
                                }
                            }, 

                            17 => { // NIR
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );

                                // Convert to values
                                let mut val: u16;
                                for j in 1..num_points_in_block {
                                    pt = point_num + j;
                                    val = bor2.read_u16().expect("Error while reading byte data.");
                                    self.colour_data[pt].nir = val;
                                }
                            }, 
                            _ => {
                                panic!("Unrecognized field code.");
                            }
                        }
                    }

                    // println!("block_bytes: {}", block_bytes);
                    point_num += num_points_in_block;
                    next_offset += block_bytes as usize;
                    if next_offset >= file_size {
                        flag = false;
                    }
                }
            } else {
                while flag {
                    bor.seek(next_offset);

                    let mut field_type = vec![];
                    let mut offset = vec![];
                    let mut num_bytes = vec![];

                    // Start by reading the point data table
                    num_fields = bor.read_u8().expect("Error while reading byte data.");
                    block_bytes = 4u64 + 20u64 * num_fields as u64;
                    compression_method = bor.read_u8().expect("Error while reading byte data.");
                    if compression_method != 0 {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Unsupported compression method.",
                        ));
                    }
                    major_version = bor.read_u8().expect("Error while reading byte data.");
                    minor_version = bor.read_u8().expect("Error while reading byte data.");

                    // Other acceptable versions include 0.0, 0.1, 1.0
                    if !(major_version == 0 && minor_version <= 1)
                        && !(major_version == 1 && minor_version == 0)
                    {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!(
                                "Unsupported ZLidar version {}.{}.",
                                major_version, minor_version
                            ),
                        ));
                    }

                    let mut return_field = -1isize;
                    for i in 0..num_fields as usize {
                        field_type.push(bor.read_u32().expect("Error while reading byte data."));
                        if field_type[i] == 4 {
                            return_field = i as isize;
                        }
                        offset.push(bor.read_u64().expect("Error while reading byte data."));
                        num_bytes.push(bor.read_u64().expect("Error while reading byte data."));
                        block_bytes += num_bytes[i];
                        // Don't forget about word alignment bytes
                        if block_bytes % 4 > 0 {
                            block_bytes += 4 - (num_bytes[i] % 4);
                        }
                        // println!("field_type: {} offset: {} num_bytes: {}", field_type[i], offset[i], num_bytes[i]);
                    }

                    // we need to read the point return data before the z-values.
                    if return_field >= 0 {
                        bor.seek(offset[return_field as usize] as usize);
                        let mut compressed = vec![0u8; num_bytes[return_field as usize] as usize];
                        bor.read_exact(&mut compressed)?;
                        let decompressed = decompress_to_vec_zlib(&compressed)
                            .expect("DEFLATE failed to decompress data.");
                        num_points_in_block = decompressed.len();
                        for j in 0..num_points_in_block {
                            pt = point_num + j;
                            self.point_data[pt].point_bit_field = decompressed[j];
                        }
                    } else {
                        return Err(Error::new(
                        ErrorKind::Other,
                        "An error was encountered while attempting to read the point return data.",
                    ));
                    }

                    for i in 0..num_fields as usize {
                        bor.seek(offset[i] as usize);
                        let mut compressed = vec![0u8; num_bytes[i] as usize];
                        bor.read_exact(&mut compressed)?;
                        let decompressed = decompress_to_vec_zlib(&compressed)
                            .expect("DEFLATE failed to decompress data.");

                        match field_type[i] {
                            0 => {
                                // x
                                num_points_in_block = decompressed.len() / 4;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                let mut val: i32;
                                let mut vali32: i32;
                                let mut prev_val = 0i32;
                                for j in 0..num_points_in_block {
                                    vali32 = bor2.read_i32().expect("Error reading byte data.");
                                    // val = (vali32 + prev_val) as f64 * self.header.x_scale_factor
                                    //     + self.header.x_offset;
                                    val = vali32 + prev_val;

                                    prev_val += vali32;
                                    pt = point_num + j;
                                    self.point_data[pt].x = val;
                                }
                            }
                            1 => {
                                // y
                                num_points_in_block = decompressed.len() / 4;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                let mut val: i32;
                                let mut vali32: i32;
                                let mut prev_val = 0i32;
                                for j in 0..num_points_in_block {
                                    vali32 = bor2.read_i32().expect("Error reading byte data.");
                                    val = vali32 + prev_val; // as f64 * self.header.y_scale_factor
                                        // + self.header.y_offset;

                                    // pt = point_num + j;
                                    // if pt >= 500_000 && pt < 500_100 {
                                    //     println!("{} {} {} {}", vali32+prev_val, vali32, prev_val, val);
                                    // }

                                    prev_val += vali32;
                                    pt = point_num + j;
                                    self.point_data[pt].y = val;
                                }
                            }
                            2 => {
                                // z
                                num_points_in_block = decompressed.len() / 4;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                let mut val: i32;
                                let mut vali32: i32;

                                // let mut prev_val = 0i32;
                                // for j in 0..num_points_in_block {
                                //     vali32 = bor2.read_i32().expect("Error reading byte data.");
                                //     val = (vali32 + prev_val) as f64 * self.header.z_scale_factor + self.header.z_offset;

                                //     pt = point_num + j;
                                //     // if pt >= 500_000 && pt < 500_100 {
                                //     //     println!("{} {} {} {}", vali32+prev_val, vali32, prev_val, val);
                                //     // }

                                //     if self.point_data[pt].is_late_return() {
                                //         prev_val += vali32;
                                //     }
                                //     self.point_data[pt].z = val;
                                // }

                                let mut prev_val = 0i32;
                                let mut prev_late_val = 0i32;
                                let mut prev_early_val = 0i32;
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    prev_val = if self.point_data[pt].is_late_return() {
                                        prev_late_val
                                    } else {
                                        prev_early_val
                                    };

                                    vali32 = bor2.read_i32().expect("Error reading byte data.");
                                    val = vali32 + prev_val; // as f64 * self.header.z_scale_factor
                                        // + self.header.z_offset;
                                    self.point_data[pt].z = val;

                                    if self.point_data[pt].is_late_return() {
                                        prev_late_val += vali32;
                                    } else {
                                        prev_early_val += vali32;
                                    }
                                }

                                // let mut prev_val = 0i32;
                                // for j in 0..num_points_in_block {
                                //     vali32 = bor2.read_i32().expect("Error reading byte data.");
                                //     val = (vali32 + prev_val) as f64 * self.header.z_scale_factor + self.header.z_offset;

                                //     pt = point_num + j;
                                //     if pt >= 500_000 && pt < 500_100 {
                                //         println!("{} {} {} {}", vali32+prev_val, vali32, prev_val, val);
                                //     }

                                //     prev_val += vali32;
                                //     // pt = point_num + j;
                                //     self.point_data[pt].z = val;
                                // }

                                // let mut prev_val: i32;
                                // let mut prev_late_val = 0i32;
                                // let mut prev_early_val = 0i32;
                                // for j in 0..num_points_in_block {
                                //     pt = point_num + j;
                                //     prev_val = if self.point_data[pt].is_late_return() {
                                //         prev_late_val
                                //     } else {
                                //         prev_early_val
                                //     };
                                //     vali32 = bor2.read_i32().expect("Error reading byte data.");
                                //     val = (vali32 + prev_val) as f64 * self.header.z_scale_factor + self.header.z_offset;
                                //     self.point_data[pt].z = val;

                                //     if self.point_data[pt].is_late_return() {
                                //         prev_late_val = ((val - self.header.z_offset) / self.header.z_scale_factor) as i32;
                                //     } else {
                                //         prev_early_val = ((val - self.header.z_offset) / self.header.z_scale_factor) as i32;
                                //     }
                                //     // if j == 1000 {
                                //     //     println!("z: {}", val);
                                //     // }
                                // }
                            }
                            3 => {
                                // intensity
                                num_points_in_block = decompressed.len() / 2;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    self.point_data[pt].intensity =
                                        bor2.read_u16().expect("Error reading byte data.");
                                }
                            }
                            4 => {
                                // point return data has already been read.
                            }
                            5 => {
                                // point class data
                                num_points_in_block = decompressed.len();
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    self.point_data[pt].class_bit_field = decompressed[j];
                                }
                            }
                            6 => {
                                // scan angle
                                num_points_in_block = decompressed.len() / 2;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                let mut val: i16;
                                let mut prev_val = 0i16;
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    val = bor2.read_i16().expect("Error reading byte data.");
                                    self.point_data[pt].scan_angle = val + prev_val;
                                    prev_val = val;
                                }
                            }
                            7 => {
                                // user data
                                num_points_in_block = decompressed.len();
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    self.point_data[pt].user_data = decompressed[j];
                                }
                            }
                            8 => {
                                // point source id
                                num_points_in_block = decompressed.len() / 2;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    self.point_data[pt].point_source_id =
                                        bor2.read_u16().expect("Error reading byte data.");
                                }
                            }
                            9 => {
                                // GPS time
                                if self.gps_data.len() == 0 {
                                    self.gps_data =
                                        vec![0f64; self.header.number_of_points as usize];
                                }
                                num_points_in_block = decompressed.len() / 8;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                let mut val: f64;
                                let mut prev_val = 0f64;
                                for j in 0..num_points_in_block {
                                    val = bor2.read_f64().expect("Error reading byte data.")
                                        + prev_val;
                                    pt = point_num + j;
                                    self.gps_data[pt] = val;
                                    prev_val = val;
                                }
                            }
                            10 => {
                                // red
                                if self.colour_data.len() == 0 {
                                    self.colour_data = vec![
                                        Default::default();
                                        self.header.number_of_points as usize
                                    ];
                                }
                                num_points_in_block = decompressed.len() / 2;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    self.colour_data[pt].red =
                                        bor2.read_u16().expect("Error reading byte data.");
                                }
                            }
                            11 => {
                                // green
                                if self.colour_data.len() == 0 {
                                    self.colour_data = vec![
                                        Default::default();
                                        self.header.number_of_points as usize
                                    ];
                                }
                                num_points_in_block = decompressed.len() / 2;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    self.colour_data[pt].green =
                                        bor2.read_u16().expect("Error reading byte data.");
                                }
                            }
                            12 => {
                                // blue
                                if self.colour_data.len() == 0 {
                                    self.colour_data = vec![
                                        Default::default();
                                        self.header.number_of_points as usize
                                    ];
                                }
                                num_points_in_block = decompressed.len() / 2;
                                let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                    Cursor::new(decompressed),
                                    Endianness::LittleEndian,
                                );
                                for j in 0..num_points_in_block {
                                    pt = point_num + j;
                                    self.colour_data[pt].blue =
                                        bor2.read_u16().expect("Error reading byte data.");
                                }
                            }
                            _ => {
                                // Do nothing // return Err(Error::new(ErrorKind::Other, "Unrecognized point field"));
                            }
                        }
                    }

                    point_num += num_points_in_block;
                    next_offset += block_bytes as usize;
                    if next_offset >= file_size {
                        flag = false;
                    }
                }
            }

            // for i in 100_000..100_200 {
            //     println!("{}, {}, {}, {}, {}, {}", i, self.point_data[i].x, self.point_data[i].y, self.point_data[i].z, self.point_data[i].return_number(), self.point_data[i].number_of_returns());
            // }
        }

        Ok(())
    }

    pub fn write(&mut self) -> Result<(), Error> {
        if self.file_mode == "r" {
            return Err(Error::new(
                ErrorKind::Other,
                "The file was opened in read-only mode",
            ));
        }
        if !self.header_is_set {
            return Err(Error::new(ErrorKind::Other, "The header of a LAS file must be added before any point records. Please see add_header()."));
        }

        // Issue a warning if there are fewer than two points in the dataset. Many tools won't work correctly if this is the case.
        if self.header.number_of_points < 2 {
            println!("WARNING: There are fewer than two points in the LAS file. This may cause some tools to fail when reading these data.");
        }

        if self.header.x_offset == f64::NEG_INFINITY {
            self.header.x_offset = self.header.min_x;
            self.header.y_offset = self.header.min_y;
            self.header.z_offset = self.header.min_z;
        }

        let mut mantissa: usize = (format!("{}", (self.header.max_x - self.header.min_x).floor()))
            .to_string()
            .len();
        let mut dec: f64 = 1.0 / 10_f64.powi(7 - mantissa as i32);
        if self.header.x_scale_factor == f64::NEG_INFINITY {
            self.header.x_scale_factor = dec;
        }

        mantissa = (format!("{}", (self.header.max_y - self.header.min_y).floor()))
            .to_string()
            .len();
        dec = 1.0 / 10_f64.powi(8 - mantissa as i32);
        if self.header.y_scale_factor == f64::NEG_INFINITY {
            self.header.y_scale_factor = dec;
        }

        mantissa = (format!("{}", (self.header.max_z - self.header.min_z).floor()))
            .to_string()
            .len();
        dec = 1.0 / 10_f64.powi(8 - mantissa as i32);
        if self.header.z_scale_factor == f64::NEG_INFINITY {
            self.header.z_scale_factor = dec;
        }

        if !self.file_name.to_lowercase().ends_with(".zip")
            && !self.file_name.to_lowercase().ends_with(".zlidar")
        {
            let f = File::create(&self.file_name)?;
            let mut writer = BufWriter::new(f);

            self.write_data(&mut writer)?;
        } else if self.file_name.to_lowercase().ends_with(".laz") {
            self.write_laz_data()?;
        } else if self.file_name.to_lowercase().ends_with(".zlidar") {
            let f = File::create(&self.file_name)?;
            let mut writer = BufWriter::new(f);
            if self.compression == ZlidarCompression::None {
                self.compression = ZlidarCompression::Brotli { level: 5u8 };
            }
            self.write_zlidar_data(&mut writer)?;
        } else {
            let f = File::create(&self.file_name)?;
            let mut writer = ZipWriter::new(f);
            let lasfile_name = if self.file_name.to_lowercase().ends_with(".las.zip") {
                let path = Path::new(&self.file_name);
                path.file_stem().unwrap().to_str().unwrap().to_owned()
            } else {
                let path = Path::new(&self.file_name);
                path.file_stem().unwrap().to_str().unwrap().to_owned() + ".las"
            };

            let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
            writer.start_file(lasfile_name, options)?;

            self.write_data(&mut writer)?;
        }

        Ok(())
    }

    fn write_data<W: Write>(&mut self, writer: &mut W) -> Result<(), Error> {
        /////////////////////////////////
        // Write the header to the file /
        /////////////////////////////////
        let mut u16_bytes: [u8; 2];
        let mut u32_bytes: [u8; 4];
        let mut u64_bytes: [u8; 8];

        self.header.file_signature = "LASF".to_string();
        writer.write_all(self.header.file_signature.as_bytes())?;

        u16_bytes = unsafe { mem::transmute(self.header.file_source_id) };
        writer.write_all(&u16_bytes)?;

        u16_bytes = unsafe { mem::transmute(self.header.global_encoding) };
        writer.write_all(&u16_bytes)?;

        if self.header.project_id_used {
            u32_bytes = unsafe { mem::transmute(self.header.project_id1) };
            writer.write_all(&u32_bytes)?;

            u16_bytes = unsafe { mem::transmute(self.header.project_id2) };
            writer.write_all(&u16_bytes)?;

            u16_bytes = unsafe { mem::transmute(self.header.project_id3) };
            writer.write_all(&u16_bytes)?;

            u64_bytes = unsafe { mem::transmute(self.header.project_id4) };
            writer.write_all(&u64_bytes)?;
        }

        self.header.version_major = 1u8;
        let mut u8_bytes: [u8; 1] = unsafe { mem::transmute(self.header.version_major) };
        writer.write_all(&u8_bytes)?;

        self.header.version_minor = 3u8;
        u8_bytes = unsafe { mem::transmute(self.header.version_minor) };
        writer.write_all(&u8_bytes)?;

        if self.header.system_id.len() == 0 {
            self.header.system_id = fixed_length_string("OTHER", 32);
        } else if !self.header.system_id.len() != 32 {
            self.header.system_id = fixed_length_string(&(self.header.system_id), 32);
        }
        writer.write_all(self.header.system_id.as_bytes())?; //string_bytes));

        self.header.generating_software =
            fixed_length_string("WhiteboxTools                   ", 32);
        writer.write_all(self.header.generating_software.as_bytes())?;

        // let now = time::now();
        // self.header.file_creation_day = now.tm_yday as u16;
        let now = Local::now();
        self.header.file_creation_day = now.ordinal() as u16;
        u16_bytes = unsafe { mem::transmute(self.header.file_creation_day) };
        writer.write_all(&u16_bytes)?;

        // self.header.file_creation_year = (now.tm_year + 1900) as u16;
        self.header.file_creation_year = now.year() as u16;
        u16_bytes = unsafe { mem::transmute(self.header.file_creation_year) };
        writer.write_all(&u16_bytes)?;

        self.header.header_size = 235; // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
        u16_bytes = unsafe { mem::transmute(self.header.header_size) };
        writer.write_all(&u16_bytes)?;

        // figure out the offset to points
        let mut total_vlr_size = 54 * self.header.number_of_vlrs;
        for i in 0..(self.header.number_of_vlrs as usize) {
            total_vlr_size += self.vlr_data[i].record_length_after_header as u32;
        }
        let alignment_bytes = 4u32 - ((self.header.header_size as u32 + total_vlr_size) % 4u32);
        self.header.offset_to_points =
            self.header.header_size as u32 + total_vlr_size + alignment_bytes; // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
        u32_bytes = unsafe { mem::transmute(self.header.offset_to_points) };
        writer.write_all(&u32_bytes)?;

        u32_bytes = unsafe { mem::transmute(self.header.number_of_vlrs) };
        writer.write_all(&u32_bytes)?;

        ////////////////////////////////////////////////////////////////////////
        // THIS NEEDS TO BE REMOVED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING //
        ////////////////////////////////////////////////////////////////////////
        self.header.point_format = match self.header.point_format {
            0u8 => 0u8,
            1u8 => 1u8,
            2u8 => 2u8,
            3u8 => 3u8,
            4u8 => {
                println!(
                    "Warning: Point Format 4 is not supported for output. Some data will be lost."
                );
                1u8
            }
            5u8 => {
                println!(
                    "Warning: Point Format 5 is not supported for output. Some data will be lost."
                );
                3u8
            }
            6u8 => 1u8,
            7u8 => 3u8,
            8u8 => {
                println!(
                    "Warning: Point Format 8 is not supported for output. Some data will be lost."
                );
                3u8
            }
            9u8 => {
                println!(
                    "Warning: Point Format 9 is not supported for output. Some data will be lost."
                );
                1u8
            }
            10u8 => {
                println!(
                    "Warning: Point Format 10 is not supported for output. Some data will be lost."
                );
                3u8
            }
            _ => {
                return Err(Error::new(ErrorKind::Other, "Unsupported point format"));
            }
        };

        u8_bytes = unsafe { mem::transmute(self.header.point_format) };
        writer.write_all(&u8_bytes)?;

        // Intensity and userdata are both optional. Figure out if they need to be read.
        // The only way to do this is to compare the point record length by point format
        let rec_lengths = [
            [20_u16, 18_u16, 19_u16, 17_u16],
            [28_u16, 26_u16, 27_u16, 25_u16],
            [26_u16, 24_u16, 25_u16, 23_u16],
            [34_u16, 32_u16, 33_u16, 31_u16],
        ];

        if self.use_point_intensity && self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][0];
        } else if !self.use_point_intensity && self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][1];
        } else if self.use_point_intensity && !self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][2];
        } else {
            //if !self.use_point_intensity && !self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][3];
        }

        u16_bytes = unsafe { mem::transmute(self.header.point_record_length) };
        writer.write_all(&u16_bytes)?;

        if self.header.number_of_points <= u32::max_value() as u64 {
            self.header.number_of_points_old = self.header.number_of_points as u32;
        // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
        } else {
            return Err(Error::new(ErrorKind::Other, "The number of points in this file requires a 64-bit format. Currently LAS 1.4 files cannot be written."));
        }
        u32_bytes = unsafe { mem::transmute(self.header.number_of_points_old) };
        writer.write_all(&u32_bytes)?;

        for i in 0..5 {
            // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
            u32_bytes = unsafe { mem::transmute(self.header.number_of_points_by_return[i] as u32) };
            writer.write_all(&u32_bytes)?;
        }

        u64_bytes = unsafe { mem::transmute(self.header.x_scale_factor) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.y_scale_factor) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.z_scale_factor) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.x_offset) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.y_offset) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.z_offset) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_x) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_x) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_y) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_y) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_z) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_z) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.waveform_data_start) };
        writer.write_all(&u64_bytes)?;

        ///////////////////////////////
        // Write the VLRs to the file /
        ///////////////////////////////
        for i in 0..(self.header.number_of_vlrs as usize) {
            let vlr = self.vlr_data[i].clone();
            u16_bytes = unsafe { mem::transmute(vlr.reserved) };
            writer.write_all(&u16_bytes)?;

            let user_id: &str = &vlr.user_id;
            //string_bytes = unsafe { mem::transmute(user_id) };
            writer.write_all(fixed_length_string(user_id, 16).as_bytes())?; //string_bytes));

            u16_bytes = unsafe { mem::transmute(vlr.record_id) };
            writer.write_all(&u16_bytes)?;

            u16_bytes = unsafe { mem::transmute(vlr.record_length_after_header) };
            writer.write_all(&u16_bytes)?;

            let description: &str = &vlr.description;
            //string_bytes = unsafe { mem::transmute(description) };
            writer.write_all(fixed_length_string(description, 32).as_bytes())?;

            writer.write_all(&vlr.binary_data)?;
        }

        ////////////////////
        // Alignment bytes /
        ////////////////////
        if alignment_bytes > 0 {
            // println!("alignment bytes: {}", alignment_bytes);
            for _ in 0..alignment_bytes {
                writer.write_all(&[0u8])?;
            }
        }

        ////////////////////////////////
        // Write the point to the file /
        ////////////////////////////////
        // let mut val: i32;
        match self.header.point_format {
            0 => {
                for i in 0..self.header.number_of_points as usize {
                    // val = ((self.point_data[i].x - self.header.x_offset)
                    //     / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].x) };
                    writer.write_all(&u32_bytes)?;

                    // val = ((self.point_data[i].y - self.header.y_offset)
                    //     / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].y) };
                    writer.write_all(&u32_bytes)?;

                    // val = ((self.point_data[i].z - self.header.z_offset)
                    //     / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].z) };
                    writer.write_all(&u32_bytes)?;

                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }

                    if !self.point_data[i].is_64bit {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(self.point_data[i].class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    } else {
                        // there is a 64-bit point in the data that we are trying to save as 32-bit.
                        let (point_bit_field, class_bit_field) =
                            self.point_data[i].get_32bit_from_64bit();

                        u8_bytes = unsafe { mem::transmute(point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u8_bytes = unsafe { mem::transmute(self.point_data[i].scan_angle as i8) };
                    writer.write_all(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].user_data) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write_all(&u16_bytes)?;
                }
            }
            1 => {
                for i in 0..self.header.number_of_points as usize {
                    // x
                    // val = ((self.point_data[i].x - self.header.x_offset)
                    //     / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].x) };
                    writer.write_all(&u32_bytes)?;
                    // y
                    // val = ((self.point_data[i].y - self.header.y_offset)
                    //     / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].y) };
                    writer.write_all(&u32_bytes)?;
                    // z
                    // val = ((self.point_data[i].z - self.header.z_offset)
                    //     / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].z) };
                    writer.write_all(&u32_bytes)?;
                    // intensity
                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }
                    if !self.point_data[i].is_64bit {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(self.point_data[i].class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    } else {
                        // there is a 64-bit point in the data that we are trying to save as 32-bit.
                        let (point_bit_field, class_bit_field) =
                            self.point_data[i].get_32bit_from_64bit();

                        u8_bytes = unsafe { mem::transmute(point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u8_bytes = unsafe { mem::transmute(self.point_data[i].scan_angle as i8) };
                    writer.write_all(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].user_data) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write_all(&u16_bytes)?;

                    u64_bytes = unsafe { mem::transmute(self.gps_data[i]) };
                    writer.write_all(&u64_bytes)?;
                }
            }
            2 => {
                for i in 0..self.header.number_of_points as usize {
                    // val = ((self.point_data[i].x - self.header.x_offset)
                    //     / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].x) };
                    writer.write_all(&u32_bytes)?;

                    // val = ((self.point_data[i].y - self.header.y_offset)
                    //     / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].y) };
                    writer.write_all(&u32_bytes)?;

                    // val = ((self.point_data[i].z - self.header.z_offset)
                    //     / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].z) };
                    writer.write_all(&u32_bytes)?;

                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }

                    if !self.point_data[i].is_64bit {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(self.point_data[i].class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    } else {
                        // there is a 64-bit point in the data that we are trying to save as 32-bit.
                        let (point_bit_field, class_bit_field) =
                            self.point_data[i].get_32bit_from_64bit();

                        u8_bytes = unsafe { mem::transmute(point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u8_bytes = unsafe { mem::transmute(self.point_data[i].scan_angle as i8) };
                    writer.write_all(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].user_data) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write_all(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.colour_data[i].red) };
                    writer.write_all(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.colour_data[i].green) };
                    writer.write_all(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.colour_data[i].blue) };
                    writer.write_all(&u16_bytes)?;
                }
            }
            3 => {
                for i in 0..self.header.number_of_points as usize {
                    // val = ((self.point_data[i].x - self.header.x_offset)
                    //     / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].x) };
                    writer.write_all(&u32_bytes)?;

                    // val = ((self.point_data[i].y - self.header.y_offset)
                    //     / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].y) };
                    writer.write_all(&u32_bytes)?;

                    // val = ((self.point_data[i].z - self.header.z_offset)
                    //     / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(self.point_data[i].z) };
                    writer.write_all(&u32_bytes)?;

                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }

                    if !self.point_data[i].is_64bit {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(self.point_data[i].class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    } else {
                        // there is a 64-bit point in the data that we are trying to save as 32-bit.
                        let (point_bit_field, class_bit_field) =
                            self.point_data[i].get_32bit_from_64bit();

                        u8_bytes = unsafe { mem::transmute(point_bit_field) };
                        writer.write_all(&u8_bytes)?;

                        u8_bytes = unsafe { mem::transmute(class_bit_field) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u8_bytes = unsafe { mem::transmute(self.point_data[i].scan_angle as i8) };
                    writer.write_all(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe { mem::transmute(self.point_data[i].user_data) };
                        writer.write_all(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write_all(&u16_bytes)?;

                    u64_bytes = unsafe { mem::transmute(self.gps_data[i]) };
                    writer.write_all(&u64_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.colour_data[i].red) };
                    writer.write_all(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.colour_data[i].green) };
                    writer.write_all(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.colour_data[i].blue) };
                    writer.write_all(&u16_bytes)?;
                }
            }
            _ => {
                return Err(Error::new(ErrorKind::Other, "Unsupported point format"));
            }
        }

        Ok(())
    }

    fn write_laz_data(&mut self) -> Result<(), Error> {

        // let mut reader = Reader::from_path(&input_file).expect("Error reading LAS file.");
        // let in_header = reader.header();
        let mut builder = Builder::from((1, 3));
        // let mut format = in_header.point_format().clone();

        let mut format = las::point::Format::new(self.header.point_format).unwrap();
        format.is_compressed = true;
        builder.point_format = format;
        builder.generating_software = "WhiteboxTools".to_string();
        let transforms: las::Vector<las::Transform> = las::Vector{ 
            x: las::Transform {scale: self.header.x_scale_factor, offset: self.header.x_offset }, 
            y: las::Transform {scale: self.header.y_scale_factor, offset: self.header.y_offset }, 
            z: las::Transform {scale: self.header.z_scale_factor, offset: self.header.z_offset }
        };
        builder.transforms = transforms.clone();
        
        for vlr in &self.vlr_data {
            let mut vlr2 = las::Vlr::default();
            vlr2.user_id = vlr.user_id.clone();
            vlr2.record_id = vlr.record_id;
            vlr2.description = vlr2.description.clone();
            vlr2.data = vlr.binary_data.clone();
        //     while vlr2.description.len() > 32 {
        //         vlr2.description.pop();
        //     }
            builder.vlrs.push(vlr2.clone());
        }

        let out_header = builder.into_header().unwrap();
        let f = File::create(&self.file_name).expect("Unable to create file");
        let f = BufWriter::new(f);
        let mut writer = OtherWriter::new(f, out_header).unwrap();
        let mut point: las::point::Point;
        let mut raw_point: las::raw::Point;
        // let mut p: Point3D;
        let mut pd: PointData;
        for point_num in 0..self.header.number_of_points as usize {
            pd = self[point_num];
            raw_point = las::raw::Point::default();

            // Coordinates and intensity information
            raw_point.x = pd.x;
            raw_point.y = pd.y;
            raw_point.z = pd.z;
            raw_point.intensity = pd.intensity;

            // Flags
            let flags = if self.header.point_format < 6 {
                las::raw::point::Flags::TwoByte(pd.point_bit_field, pd.class_bit_field)
            } else {
                las::raw::point::Flags::ThreeByte(pd.point_bit_field, pd.class_bit_field, pd.classification)
            };
            raw_point.flags = flags;

            raw_point.user_data = pd.user_data;

            if self.header.point_format < 6 {
                raw_point.scan_angle = las::raw::point::ScanAngle::Rank(pd.scan_angle as i8);
            } else {
                raw_point.scan_angle = las::raw::point::ScanAngle::Scaled(pd.scan_angle);
            }
            
            raw_point.point_source_id = pd.point_source_id;

            // GPS time information
            if self.has_gps_time() {
                raw_point.gps_time = Some(self.gps_data[point_num]);
            } else {
                raw_point.gps_time = None;
            }

            // Colour information
            if self.has_rgb() {
                let colour = las::Color { 
                    red: self.colour_data[point_num].red,
                    green: self.colour_data[point_num].green,
                    blue: self.colour_data[point_num].blue
                };
                raw_point.color = Some(colour);

                if self.header.point_format == 8 || self.header.point_format == 10 {
                    raw_point.nir = Some(self.colour_data[point_num].nir);
                } else {
                    raw_point.nir = None;
                }
            } else {
                raw_point.color = None;
            }

            // Waveform information
            if self.header.point_format == 4 || self.header.point_format == 5 ||
            self.header.point_format == 9 || self.header.point_format == 10 {
                let wf = las::raw::point::Waveform {
                    wave_packet_descriptor_index: self.waveform_data[point_num].packet_descriptor_index,
                    byte_offset_to_waveform_data: self.waveform_data[point_num].offset_to_waveform_data,
                    waveform_packet_size_in_bytes: self.waveform_data[point_num].waveform_packet_size,
                    return_point_waveform_location: self.waveform_data[point_num].ret_point_waveform_loc,
                    x_t: self.waveform_data[point_num].xt,
                    y_t: self.waveform_data[point_num].yt,
                    z_t: self.waveform_data[point_num].zt,
                };
                raw_point.waveform = Some(wf);
            } else {
                raw_point.waveform = None;
            }

            point = las::point::Point::new(raw_point, &transforms);
            writer.write(point.clone()).expect("Error writing point data");
        }

        writer.close().unwrap();
        
        Ok(())
    }

    fn write_zlidar_data<W: Write>(&mut self, writer: &mut W) -> Result<(), Error> {
        /////////////////////////////////
        // Write the header to the file /
        /////////////////////////////////
        let mut u16_bytes: [u8; 2];
        let mut u32_bytes: [u8; 4];
        let mut u64_bytes: [u8; 8];

        self.header.file_signature = "ZLDR".to_string();
        writer.write_all(self.header.file_signature.as_bytes())?;

        u16_bytes = unsafe { mem::transmute(self.header.file_source_id) };
        writer.write_all(&u16_bytes)?;

        u16_bytes = unsafe { mem::transmute(self.header.global_encoding) };
        writer.write_all(&u16_bytes)?;

        if self.header.project_id_used {
            u32_bytes = unsafe { mem::transmute(self.header.project_id1) };
            writer.write_all(&u32_bytes)?;

            u16_bytes = unsafe { mem::transmute(self.header.project_id2) };
            writer.write_all(&u16_bytes)?;

            u16_bytes = unsafe { mem::transmute(self.header.project_id3) };
            writer.write_all(&u16_bytes)?;

            u64_bytes = unsafe { mem::transmute(self.header.project_id4) };
            writer.write_all(&u64_bytes)?;
        }

        self.header.version_major = 1u8;
        let mut u8_bytes: [u8; 1] = unsafe { mem::transmute(self.header.version_major) };
        writer.write_all(&u8_bytes)?;

        self.header.version_minor = 3u8;
        u8_bytes = unsafe { mem::transmute(self.header.version_minor) };
        writer.write_all(&u8_bytes)?;

        if self.header.system_id.len() == 0 {
            self.header.system_id = fixed_length_string("OTHER", 32);
        } else if !self.header.system_id.len() != 32 {
            self.header.system_id = fixed_length_string(&(self.header.system_id), 32);
        }
        writer.write_all(self.header.system_id.as_bytes())?; //string_bytes));

        self.header.generating_software =
            fixed_length_string("WhiteboxTools                   ", 32);
        writer.write_all(self.header.generating_software.as_bytes())?;

        // let now = time::now();
        // self.header.file_creation_day = now.tm_yday as u16;
        let now = Local::now();
        self.header.file_creation_day = now.ordinal() as u16;
        u16_bytes = unsafe { mem::transmute(self.header.file_creation_day) };
        writer.write_all(&u16_bytes)?;

        // self.header.file_creation_year = (now.tm_year + 1900) as u16;
        self.header.file_creation_year = now.year() as u16;
        u16_bytes = unsafe { mem::transmute(self.header.file_creation_year) };
        writer.write_all(&u16_bytes)?;

        self.header.header_size = 235; // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
        u16_bytes = unsafe { mem::transmute(self.header.header_size) };
        writer.write_all(&u16_bytes)?;

        // figure out the offset to points
        let mut total_vlr_size = 54 * self.header.number_of_vlrs;
        for i in 0..(self.header.number_of_vlrs as usize) {
            total_vlr_size += self.vlr_data[i].record_length_after_header as u32;
        }
        let alignment_bytes = 4u32 - ((self.header.header_size as u32 + total_vlr_size) % 4u32);
        self.header.offset_to_points =
            self.header.header_size as u32 + total_vlr_size + alignment_bytes; // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
        u32_bytes = unsafe { mem::transmute(self.header.offset_to_points) };
        writer.write_all(&u32_bytes)?;

        u32_bytes = unsafe { mem::transmute(self.header.number_of_vlrs) };
        writer.write_all(&u32_bytes)?;

        ////////////////////////////////////////////////////////////////////////
        // THIS NEEDS TO BE REMOVED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING //
        ////////////////////////////////////////////////////////////////////////
        self.header.point_format = match self.header.point_format {
            0u8 => 0u8,
            1u8 => 1u8,
            2u8 => 2u8,
            3u8 => 3u8,
            4u8 => {
                println!(
                    "Warning: Point Format 4 is not supported for output. Some data will be lost."
                );
                1u8
            }
            5u8 => {
                println!(
                    "Warning: Point Format 5 is not supported for output. Some data will be lost."
                );
                3u8
            }
            6u8 => 1u8,
            7u8 => 3u8,
            8u8 => {
                println!(
                    "Warning: Point Format 8 is not supported for output. Some data will be lost."
                );
                3u8
            }
            9u8 => {
                println!(
                    "Warning: Point Format 9 is not supported for output. Some data will be lost."
                );
                1u8
            }
            10u8 => {
                println!(
                    "Warning: Point Format 10 is not supported for output. Some data will be lost."
                );
                3u8
            }
            _ => {
                return Err(Error::new(ErrorKind::Other, "Unsupported point format"));
            }
        };

        u8_bytes = unsafe { mem::transmute(self.header.point_format) };
        writer.write_all(&u8_bytes)?;

        // Intensity and userdata are both optional. Figure out if they need to be read.
        // The only way to do this is to compare the point record length by point format
        let rec_lengths = [
            [20_u16, 18_u16, 19_u16, 17_u16],
            [28_u16, 26_u16, 27_u16, 25_u16],
            [26_u16, 24_u16, 25_u16, 23_u16],
            [34_u16, 32_u16, 33_u16, 31_u16],
        ];

        if self.use_point_intensity && self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][0];
        } else if !self.use_point_intensity && self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][1];
        } else if self.use_point_intensity && !self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][2];
        } else {
            //if !self.use_point_intensity && !self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][3];
        }

        u16_bytes = unsafe { mem::transmute(self.header.point_record_length) };
        writer.write_all(&u16_bytes)?;

        if self.header.number_of_points <= u32::max_value() as u64 {
            self.header.number_of_points_old = self.header.number_of_points as u32;
        // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
        } else {
            return Err(Error::new(ErrorKind::Other, "The number of points in this file requires a 64-bit format. Currently LAS 1.4 files cannot be written."));
        }
        u32_bytes = unsafe { mem::transmute(self.header.number_of_points_old) };
        writer.write_all(&u32_bytes)?;

        for i in 0..5 {
            // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
            u32_bytes = unsafe { mem::transmute(self.header.number_of_points_by_return[i] as u32) };
            writer.write_all(&u32_bytes)?;
        }

        u64_bytes = unsafe { mem::transmute(self.header.x_scale_factor) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.y_scale_factor) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.z_scale_factor) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.x_offset) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.y_offset) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.z_offset) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_x) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_x) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_y) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_y) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_z) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_z) };
        writer.write_all(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.waveform_data_start) };
        writer.write_all(&u64_bytes)?;

        ///////////////////////////////
        // Write the VLRs to the file /
        ///////////////////////////////
        for i in 0..(self.header.number_of_vlrs as usize) {
            let vlr = self.vlr_data[i].clone();
            u16_bytes = unsafe { mem::transmute(vlr.reserved) };
            writer.write_all(&u16_bytes)?;

            let user_id: &str = &vlr.user_id;
            //string_bytes = unsafe { mem::transmute(user_id) };
            writer.write_all(fixed_length_string(user_id, 16).as_bytes())?; //string_bytes));

            u16_bytes = unsafe { mem::transmute(vlr.record_id) };
            writer.write_all(&u16_bytes)?;

            u16_bytes = unsafe { mem::transmute(vlr.record_length_after_header) };
            writer.write_all(&u16_bytes)?;

            let description: &str = &vlr.description;
            //string_bytes = unsafe { mem::transmute(description) };
            writer.write_all(fixed_length_string(description, 32).as_bytes())?;

            writer.write_all(&vlr.binary_data)?;
        }

        ////////////////////
        // Alignment bytes /
        ////////////////////
        if alignment_bytes > 0 {
            // println!("alignment bytes: {}", alignment_bytes);
            for _ in 0..alignment_bytes {
                writer.write_all(&[0u8])?;
            }
        }

        /////////////////////////////////////
        // Write the point data to the file /
        /////////////////////////////////////

        // Write the zlidar header

        // Number of fields in point blocks
        let mut num_fields = match self.header.point_format {
            2 | 3 | 5 | 7 => 15u8, // includes RGB
            8 => 16u8,             // includes RGB and NIR
            _ => 12u8,             // does not include any colour
        };

        if self.use_point_intensity {
            num_fields += 1;
        }
        if self.use_point_userdata {
            num_fields += 1;
        }

        writer
            .write_u8(num_fields)
            .expect("Error writing byte data to file.");

        // Compression method
        // let compression_method = 1; // 0 = DEFLATE, 1 = brotli
        // let compression_level = 5;
        // match compression_method {
        //     0 => println!("Compression method: DEFLATE-{}", compression_level),
        //     1 => println!("Compression method: Brotli-{}", compression_level),
        //     _ => println!("Compression method: not recognized"),
        // }

        let compression_method = match self.compression {
            ZlidarCompression::Deflate { level: _ } => {
                0u8
            },
            ZlidarCompression::Brotli { level: _ } => {
                1u8
            }
            _ => { 1u8 }
        };

        let compression_level = match self.compression {
            ZlidarCompression::Deflate { level } => {
                level
            },
            ZlidarCompression::Brotli { level } => {
                level
            }
            _ => { 5u8 }
        };

        let compression_byte = ((compression_level & 0b0001_1111) << 3) | (compression_method & 0b000_0111);
        
        writer
            .write_u8(compression_byte)
            .expect("Error writing byte data to file.");

        // writer
        //     .write_u8(compression_level as u8)
        //     .expect("Error writing byte data to file.");

        // Version major and minor
        writer
            .write_u8(1u8)
            .expect("Error writing byte data to file."); // zlidar major version number

        writer
            .write_u8(1u8)
            .expect("Error writing byte data to file."); // zlidar minor version number

        // Now write the point data blocks
        let mut current_offset = self.header.offset_to_points as u64 + 4; // the four bytes are for the zlidar header
        let mut data_length_in_bytes: u64;
        let mut val: i32;
        let mut val2: i32;
        let mut val_u8: u8;
        let mut num_bits: i32;
        let mut num_bits_in_byte: i32;
        // let mut b: u8;
        let mut ret_num_diff: i16;
        let mut scanner_chan: usize;
        let mut cntx: usize;
        let mut tag: u8;
        let block_size = 50_000usize;
        let mut block_start = 0usize;
        let mut block_end = block_size;
        if block_end > self.header.number_of_points as usize {
            block_end = self.header.number_of_points as usize;
        }
        while block_start < self.header.number_of_points as usize {
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];

            // Change byte
            writer.write_u8(0u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut change_data = Vec::with_capacity(block_size);
            change_data.push(0u8);
            // val_u8 = 0b0111_1111u8;
            for i in block_start + 1..block_end {
                val_u8 = 0;
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].scanner_channel() != self.point_data[i - 1].scanner_channel()
                {
                    val_u8 = val_u8 | 0b0000_0001u8;
                }

                if self.gps_data.len() > 0
                    && self.gps_data[i] != self.gps_data[scanner_chan_index[scanner_chan]]
                {
                    val_u8 = val_u8 | 0b0000_0010u8;
                }

                ret_num_diff = self.point_data[i].return_number() as i16
                    - self.point_data[scanner_chan_index[scanner_chan]].return_number() as i16;
                if ret_num_diff == 1 {
                    val_u8 = val_u8 | 0b0000_0100u8;
                } else if ret_num_diff == -1 {
                    val_u8 = val_u8 | 0b0000_1000u8;
                } else if ret_num_diff != 0 {
                    val_u8 = val_u8 | 0b0000_1100u8;
                }

                if self.point_data[i].number_of_returns()
                    != self.point_data[scanner_chan_index[scanner_chan]].number_of_returns()
                {
                    val_u8 = val_u8 | 0b0001_0000u8;
                }

                if self.point_data[i].classification()
                    != self.point_data[scanner_chan_index[scanner_chan]].classification()
                {
                    val_u8 = val_u8 | 0b0010_0000u8;
                }

                if self.point_data[i].scan_angle
                    != self.point_data[scanner_chan_index[scanner_chan]].scan_angle
                {
                    val_u8 = val_u8 | 0b0100_0000u8;
                }

                if self.point_data[i].intensity > 255 {
                    val_u8 = val_u8 | 0b1000_0000u8;
                }

                change_data.push(val_u8);

                scanner_chan_index[scanner_chan] = i;
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&change_data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(change_data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            // println!("{} {}, {:?}", current_offset, data_length_in_bytes, compressed_data);
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            // Scanner channel
            writer.write_u8(1u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size / 4 + 1);
            val_u8 = self.point_data[block_start].scanner_channel();
            num_bits = 2;
            num_bits_in_byte = num_bits;
            for i in block_start + 1..block_end {
                if self.point_data[i].scanner_channel() != self.point_data[i - 1].scanner_channel()
                {
                    val_u8 = val_u8 | (self.point_data[i].scanner_channel() << num_bits_in_byte);
                    num_bits_in_byte += num_bits;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8);
                        val_u8 = 0;
                        num_bits_in_byte = 0;
                    }
                }
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }

            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            // Return number
            writer.write_u8(2u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size);
            val_u8 = self.point_data[block_start].return_number();
            num_bits = 4;
            num_bits_in_byte = num_bits;
            scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start + 1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                ret_num_diff = self.point_data[i].return_number() as i16
                    - self.point_data[scanner_chan_index[scanner_chan]].return_number() as i16;
                if ret_num_diff.abs() > 1 {
                    val_u8 = val_u8 | (self.point_data[i].return_number() << num_bits_in_byte);
                    num_bits_in_byte += num_bits;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8);
                        val_u8 = 0;
                        num_bits_in_byte = 0;
                    }
                }
                scanner_chan_index[scanner_chan] = i;
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }

            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            // Number of returns
            writer.write_u8(3u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size);
            val_u8 = self.point_data[block_start].number_of_returns();
            // println!("{} {}", block_start, val_u8);
            num_bits = 4;
            num_bits_in_byte = num_bits;
            scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start + 1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].number_of_returns()
                    != self.point_data[scanner_chan_index[scanner_chan]].number_of_returns()
                {
                    val_u8 = val_u8 | (self.point_data[i].number_of_returns() << num_bits_in_byte);
                    num_bits_in_byte += num_bits;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8);
                        val_u8 = 0;
                        num_bits_in_byte = 0;
                    }
                }
                scanner_chan_index[scanner_chan] = i;

                // if i >= 100_000 && i < 100_100 {
                // // if i >= 0 && i < 100 {
                //     println!("{}, {}/{}", i, self.point_data[i].return_number(), self.point_data[i].number_of_returns());
                // }
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }

            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;


            // x
            writer.write_u8(4u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut delta_values = Vec::with_capacity(block_size);
            let mut prev_vals = [0i32, 0i32, 0i32, 0i32];
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                // val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor)
                //     as i32;
                val = self.point_data[i].x;
                delta_values.push(val - prev_vals[scanner_chan]);
                prev_vals[scanner_chan] = val;
            }

            let mut data = Vec::with_capacity(block_size / 4 + 1);
            let mut data2 = Vec::with_capacity(block_size * 4);
            let mut prev_index = [
                [block_start; 16],
                [block_start; 16],
                [block_start; 16],
                [block_start; 16],
            ];
            tag = 0u8;
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                val = delta_values[i - block_start];
                cntx = self.get_context(i);
                val2 = val - delta_values[prev_index[scanner_chan][cntx] - block_start];

                if i > block_start {
                    tag = if val2.abs() <= 6 {
                        // half-byte
                        // histo_x[0] += 1;
                        (val2 + 6) as u8
                    } else if val2 >= -128 && val2 <= 127 {
                        // 1 byte
                        // histo_x[1] += 1;
                        data2
                            .write_i8(val2 as i8)
                            .expect("Error writing byte data.");
                        13u8
                    } else if val2 >= -32768 && val2 <= 32767 {
                        // 2 bytes
                        // histo_x[2] += 1;
                        data2
                            .write_i16::<LittleEndian>(val2 as i16)
                            .expect("Error writing byte data.");
                        14u8
                    } else {
                        // 4 bytes
                        // histo_x[3] += 1;
                        data2
                            .write_i32::<LittleEndian>(val2)
                            .expect("Error writing byte data.");
                        15u8
                    };

                    // if i >= 100_000 && i < 100_100 {
                    //     println!("{}, {}/{}, {}, {}, {}", i, self.point_data[i].return_number(), self.point_data[i].number_of_returns(), val2, delta_values[i - block_start], self.point_data[i].x);
                    // }

                    val_u8 = val_u8 | (tag << num_bits_in_byte);
                    num_bits_in_byte += 4;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8); // flush it
                        val_u8 = 0; // reset it
                        num_bits_in_byte = 0; // reset it
                    }
                } else {
                    val_u8 = 15u8;
                    num_bits_in_byte = 4;
                    data2
                        .write_i32::<LittleEndian>(val)
                        .expect("Error writing byte data.");
                }

                prev_index[scanner_chan][cntx] = i;
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8); // flush it
            }

            let compressed_change_bits = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data2, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data2.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to change byte
            current_offset += 8;
            data_length_in_bytes = compressed_change_bits.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to change byte
            current_offset += 8 as u64;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_change_bits.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_change_bits)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            // y
            writer.write_u8(5u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut delta_values = Vec::with_capacity(block_size);
            let mut prev_vals = [0i32, 0i32, 0i32, 0i32];
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                // val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor)
                //     as i32;
                val = self.point_data[i].y;
                delta_values.push(val - prev_vals[scanner_chan]);
                prev_vals[scanner_chan] = val;
            }

            let mut data = Vec::with_capacity(block_size / 4 + 1);
            let mut data2 = Vec::with_capacity(block_size * 4);
            let mut prev_index = [
                [block_start; 16],
                [block_start; 16],
                [block_start; 16],
                [block_start; 16],
            ];
            tag = 0u8;
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                val = delta_values[i - block_start];
                cntx = self.get_context(i);
                val2 = val - delta_values[prev_index[scanner_chan][cntx] - block_start];

                if i > block_start {
                    tag = if val2.abs() <= 6 {
                        // half-byte
                        // histo_x[0] += 1;
                        (val2 + 6) as u8
                    } else if val2 >= -128 && val2 <= 127 {
                        // 1 byte
                        // histo_x[1] += 1;
                        data2
                            .write_i8(val2 as i8)
                            .expect("Error writing byte data.");
                        13u8
                    } else if val2 >= -32768 && val2 <= 32767 {
                        // 2 bytes
                        // histo_x[2] += 1;
                        data2
                            .write_i16::<LittleEndian>(val2 as i16)
                            .expect("Error writing byte data.");
                        14u8
                    } else {
                        // 4 bytes
                        // histo_x[3] += 1;
                        data2
                            .write_i32::<LittleEndian>(val2)
                            .expect("Error writing byte data.");
                        15u8
                    };

                    val_u8 = val_u8 | (tag << num_bits_in_byte);
                    num_bits_in_byte += 4;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8); // flush it
                        val_u8 = 0; // reset it
                        num_bits_in_byte = 0; // reset it
                    }

                    // if i >= 100_000 && i < 100_100 {
                    //     println!("{}, {}, {}", i, self.point_data[i].x, self.point_data[i].y);
                    // }
                } else {
                    val_u8 = 15u8;
                    num_bits_in_byte = 4;
                    data2
                        .write_i32::<LittleEndian>(val)
                        .expect("Error writing byte data.");
                }

                prev_index[scanner_chan][cntx] = i;
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8); // flush it
            }

            let compressed_change_bits = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data2, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data2.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to change byte
            current_offset += 8;
            data_length_in_bytes = compressed_change_bits.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to change byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_change_bits.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_change_bits)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;


            // z
            writer.write_u8(6u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;
            let mut data = Vec::with_capacity(block_size / 4 + 1);
            let mut data2 = Vec::with_capacity(block_size * 4);
            let mut prev_late_vals = [0i32, 0i32, 0i32, 0i32];
            let mut prev_early_vals = [0i32, 0i32, 0i32, 0i32];
            let mut prev_val = 0i32;
            tag = 0u8;
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                prev_val = if self.point_data[i].is_late_return() {
                    prev_late_vals[scanner_chan]
                } else {
                    prev_early_vals[scanner_chan]
                };
                // val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor)
                //     as i32;
                val = self.point_data[i].z;
                val2 = val - prev_val;

                if i > block_start {
                    tag = if val2.abs() <= 6 {
                        // half-byte
                        (val2 + 6) as u8
                    } else if val2 >= -128 && val2 <= 127 {
                        // 1 byte
                        data2
                            .write_i8(val2 as i8)
                            .expect("Error writing byte data.");
                        13u8 // tag for a single signed byte
                    } else if val2 >= -32768 && val2 <= 32767 {
                        // 2 bytes
                        data2
                            .write_i16::<LittleEndian>(val2 as i16)
                            .expect("Error writing byte data.");
                        14u8 // tag for a 2-byte signed int
                    } else {
                        // 4 bytes
                        data2
                            .write_i32::<LittleEndian>(val2)
                            .expect("Error writing byte data.");
                        15u8 // tag for a 4-byte signed int
                    };

                    val_u8 = val_u8 | (tag << num_bits_in_byte);
                    num_bits_in_byte += 4;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8); // flush it
                        val_u8 = 0; // reset it
                        num_bits_in_byte = 0; // reset it
                    }
                } else {
                    val_u8 = 15u8;
                    num_bits_in_byte = 4;
                    data2
                        .write_i32::<LittleEndian>(val2)
                        .expect("Error writing byte data.");
                }

                // if i >= 100_000 && i < 100_100 {
                //     println!("{}, {}, {}, {}", i, self.point_data[i].x, self.point_data[i].y, self.point_data[i].z);
                // }

                if self.point_data[i].is_late_return() {
                    prev_late_vals[scanner_chan] = val;
                } else {
                    prev_early_vals[scanner_chan] = val;
                }
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8); // flush it
            }

            let compressed_change_bits = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data2, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data2.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to change byte
            current_offset += 8;
            data_length_in_bytes = compressed_change_bits.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to change byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_change_bits.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_change_bits)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;
            
            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;


            // intensity
            if self.use_point_intensity {
                writer.write_u8(7u8).expect("Error writing byte data to file."); // Field code
                current_offset += 1;

                let mut data2 = Vec::with_capacity(block_size * 2);
                let mut val: u16;
                data2.write_u16::<LittleEndian>(self.point_data[block_start].intensity).expect("Error writing byte data.");
                for i in block_start+1..block_end {
                    val = self.point_data[i].intensity;
                    if val < 256 {
                        data2.write_u8(val as u8).expect("Error writing byte data.");
                    } else {
                        data2
                            .write_u16::<LittleEndian>(val)
                            .expect("Error writing byte data.");
                    }

                    // if i >= 100_000 && i < 100_100 {
                    //     let t = val > 255;
                    //     println!("{}, {}, {}", i, t, val);
                    // }
                }

                let compressed_data = if compression_method == 0 {
                    // DEFLATE
                    compress_to_vec_zlib(&data2, compression_level)
                } else if compression_method == 1 {
                    // brotli
                    brotli_compress(data2.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };

                writer
                    .write_u64::<LittleEndian>(current_offset + 16)
                    .expect("Error writing byte data to file."); // FileOffset to data byte
                current_offset += 8;
                data_length_in_bytes = compressed_data.len() as u64;
                writer
                    .write_u64::<LittleEndian>(data_length_in_bytes)
                    .expect("Error writing byte data to file."); // ByteLength to data byte
                current_offset += 8;
                // byte alignment to words.
                // if data_length_in_bytes % 4 > 0 {
                //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
                //         compressed_data.push(0u8);
                //         current_offset += 1;
                //     }
                // }
                writer
                    .write_all(&compressed_data)
                    .expect("Error writing byte data to file.");
                current_offset += data_length_in_bytes as u64;
            }

            // Flags
            writer.write_u8(8u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size);
            // val_u8 = 0;
            // if self.point_data[block_start].synthetic() {
            //     val_u8 = val_u8 | 0b0000_0001u8;
            // }
            // if self.point_data[block_start].keypoint() {
            //     val_u8 = val_u8 | 0b0000_0010u8;
            // }
            // if self.point_data[block_start].withheld() {
            //     val_u8 = val_u8 | 0b0000_0100u8;
            // }
            // if self.point_data[block_start].overlap() {
            //     val_u8 = val_u8 | 0b0000_1000u8;
            // }
            // if self.point_data[block_start].scan_direction_flag() {
            //     val_u8 = val_u8 | 0b0001_0000u8;
            // }
            // if self.point_data[block_start].edge_of_flightline_flag() {
            //     val_u8 = val_u8 | 0b0010_0000u8;
            // }
            // data.push(val_u8);
            for i in block_start..block_end {
                val_u8 = 0;
                if self.point_data[i].synthetic() {
                    val_u8 = val_u8 | 0b0000_0001u8;
                }
                if self.point_data[i].keypoint() {
                    val_u8 = val_u8 | 0b0000_0010u8;
                }
                if self.point_data[i].withheld() {
                    val_u8 = val_u8 | 0b0000_0100u8;
                }
                if self.point_data[i].overlap() {
                    val_u8 = val_u8 | 0b0000_1000u8;
                }
                if self.point_data[i].scan_direction_flag() {
                    val_u8 = val_u8 | 0b0001_0000u8;
                }
                if self.point_data[i].edge_of_flightline_flag() {
                    val_u8 = val_u8 | 0b0010_0000u8;
                }
                data.push(val_u8);
            }
            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            // Classification byte
            writer.write_u8(9u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size);
            data.push(self.point_data[block_start].classification());
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start + 1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].classification()
                    != self.point_data[scanner_chan_index[scanner_chan]].classification()
                {
                    data.push(self.point_data[i].classification());
                }
                scanner_chan_index[scanner_chan] = i;

                // if i >= 100_000 && i < 100_100 {
                //     println!("{}, {}", i, self.point_data[i].classification());
                // }
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            // user data
            if self.use_point_userdata {
                writer.write_u8(10u8).expect("Error writing byte data to file."); // Field code
                current_offset += 1;

                let mut data = Vec::with_capacity(block_size / 8 + 1);
                val_u8 = 0b0000_0001;
                num_bits = 1;
                num_bits_in_byte = num_bits;
                scanner_chan_index = [block_start, block_start, block_start, block_start];
                for i in block_start + 1..block_end {
                    scanner_chan = self.point_data[i].scanner_channel() as usize;
                    if self.point_data[i].user_data
                        != self.point_data[scanner_chan_index[scanner_chan]].user_data
                    {
                        val_u8 = val_u8 | (0b0000_0001 << num_bits_in_byte);
                    }
                    num_bits_in_byte += num_bits;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8);
                        val_u8 = 0;
                        num_bits_in_byte = 0;
                    }
                    scanner_chan_index[scanner_chan] = i;
                }
                if num_bits_in_byte > 0 {
                    data.push(val_u8);
                }
                let compressed_change_bits = if compression_method == 0 {
                    // DEFLATE
                    compress_to_vec_zlib(&data, compression_level)
                } else if compression_method == 1 {
                    // brotli
                    brotli_compress(data.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };

                let mut data = Vec::with_capacity(block_size);
                data.push(self.point_data[block_start].user_data);
                let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
                for i in block_start + 1..block_end {
                    scanner_chan = self.point_data[i].scanner_channel() as usize;
                    if self.point_data[i].user_data
                        != self.point_data[scanner_chan_index[scanner_chan]].user_data
                    {
                        data.push(self.point_data[i].user_data);
                    }
                    scanner_chan_index[scanner_chan] = i;

                    // if i >= 100_000 && i < 100_100 {
                    //     println!("{}, {}", i, self.point_data[i].user_data);
                    // }
                }

                let compressed_data = if compression_method == 0 {
                    // DEFLATE
                    compress_to_vec_zlib(&data, compression_level)
                } else if compression_method == 1 {
                    // brotli
                    brotli_compress(data.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };

                writer
                    .write_u64::<LittleEndian>(current_offset + 16)
                    .expect("Error writing byte data to file."); // FileOffset to change byte
                current_offset += 8;
                data_length_in_bytes = compressed_change_bits.len() as u64;
                writer
                    .write_u64::<LittleEndian>(data_length_in_bytes)
                    .expect("Error writing byte data to file."); // ByteLength to change byte
                current_offset += 8;
                // byte alignment to words.
                // if data_length_in_bytes % 4 > 0 {
                //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
                //         compressed_change_bits.push(0u8);
                //         current_offset += 1;
                //     }
                // }
                writer
                    .write_all(&compressed_change_bits)
                    .expect("Error writing byte data to file.");
                current_offset += data_length_in_bytes as u64;
                
                writer
                    .write_u64::<LittleEndian>(current_offset + 16)
                    .expect("Error writing byte data to file."); // FileOffset to data byte
                current_offset += 8;
                data_length_in_bytes = compressed_data.len() as u64;
                writer
                    .write_u64::<LittleEndian>(data_length_in_bytes)
                    .expect("Error writing byte data to file."); // ByteLength to data byte
                current_offset += 8;
                // byte alignment to words.
                // if data_length_in_bytes % 4 > 0 {
                //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
                //         compressed_data.push(0u8);
                //         current_offset += 1;
                //     }
                // }
                writer
                    .write_all(&compressed_data)
                    .expect("Error writing byte data to file.");
                current_offset += data_length_in_bytes as u64;
            }

            // Scan angle
            writer.write_u8(11u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size * 2);
            data.write_i16::<LittleEndian>(self.point_data[block_start].scan_angle)
                .expect("Error writing byte data.");
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start + 1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].scan_angle
                    != self.point_data[scanner_chan_index[scanner_chan]].scan_angle
                {
                    data.write_i16::<LittleEndian>(self.point_data[i].scan_angle)
                        .expect("Error writing byte data.");
                }
                scanner_chan_index[scanner_chan] = i;

                // if i >= 100_000 && i < 100_100 {
                //     println!("{}, {}", i, self.point_data[i].scan_angle);
                // }
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;


            // point_source_id
            writer.write_u8(12u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size / 8 + 1);
            val_u8 = 0b0000_0001;
            num_bits = 1;
            num_bits_in_byte = num_bits;
            scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start + 1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].point_source_id
                    != self.point_data[scanner_chan_index[scanner_chan]].point_source_id
                {
                    val_u8 = val_u8 | (0b0000_0001 << num_bits_in_byte);
                }
                num_bits_in_byte += num_bits;
                if num_bits_in_byte == 8 {
                    data.push(val_u8);
                    val_u8 = 0;
                    num_bits_in_byte = 0;
                }
                scanner_chan_index[scanner_chan] = i;
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }
            let compressed_change_bits = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            let mut data = Vec::with_capacity(block_size * 2);
            data.write_u16::<LittleEndian>(self.point_data[block_start].point_source_id)
                .expect("Error writing byte data.");
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start + 1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].point_source_id
                    != self.point_data[scanner_chan_index[scanner_chan]].point_source_id
                {
                    data.write_u16::<LittleEndian>(self.point_data[i].point_source_id)
                        .expect("Error writing byte data.");
                }
                scanner_chan_index[scanner_chan] = i;

                // if i >= 100_000 && i < 100_100 {
                //     println!("{}, {}", i, self.point_data[i].point_source_id);
                // }
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to change byte
            current_offset += 8;
            data_length_in_bytes = compressed_change_bits.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to change byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_change_bits.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_change_bits)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;
            
            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;

            // GPS time
            writer.write_u8(13u8).expect("Error writing byte data to file."); // Field code
            current_offset += 1;

            let mut data = Vec::with_capacity(block_size * 8);
            if self.gps_data.len() > 0 {
                data.write_f64::<LittleEndian>(self.gps_data[block_start])
                    .expect("Error writing byte data.");
            } else {
                data.write_f64::<LittleEndian>(0f64)
                    .expect("Error writing byte data.");
            };
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start + 1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.gps_data.len() > 0
                    && self.gps_data[i] != self.gps_data[scanner_chan_index[scanner_chan]]
                {
                    data.write_f64::<LittleEndian>(
                        self.gps_data[i] - self.gps_data[scanner_chan_index[scanner_chan]],
                    )
                    .expect("Error writing byte data.");
                }

                // if self.gps_data.len() > 0 {
                //     cntx = self.get_context(i);
                //     if dt[i - block_start] != dt[prev_index[scanner_chan][cntx]-block_start] {
                //         data.write_i64::<LittleEndian>(dt[i - block_start]).expect("Error writing byte data.");
                //     }
                //     prev_index[scanner_chan][cntx] = i;
                // }

                // if self.gps_data.len() > 0 && delta_values[i - block_start] != 0i64 {
                //     cntx = self.get_context(i);
                //     time_diff = delta_values[i - block_start] - delta_values[scanner_chan_index[scanner_chan] - block_start];
                //     if i >= 500_000 && i < 500_050 {
                //         println!("{} {}", time_diff, self.get_short_filename());
                //     }
                //     data.write_i64::<LittleEndian>(time_diff).expect("Error writing byte data.");
                //     // time_diff = self.gps_data[i] - self.gps_data[scanner_chan_index[scanner_chan]];
                //     // if time_diff != 0f64 && time_diff.abs() != time_interval && time_diff.abs() != two_time_interval {
                //     //     data.write_f64::<LittleEndian>(time_diff).expect("Error writing byte data.");
                //     // }
                // }

                scanner_chan_index[scanner_chan] = i;

                // if i >= 100_000 && i < 100_100 {
                //     println!("{}, {}", i, self.gps_data[i]);
                // }
            }

            let compressed_data = if compression_method == 0 {
                // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 {
                // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };

            writer
                .write_u64::<LittleEndian>(current_offset + 16)
                .expect("Error writing byte data to file."); // FileOffset to data byte
            current_offset += 8;
            data_length_in_bytes = compressed_data.len() as u64;
            writer
                .write_u64::<LittleEndian>(data_length_in_bytes)
                .expect("Error writing byte data to file."); // ByteLength to data byte
            current_offset += 8;
            // byte alignment to words.
            // if data_length_in_bytes % 4 > 0 {
            //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
            //         compressed_data.push(0u8);
            //         current_offset += 1;
            //     }
            // }
            writer
                .write_all(&compressed_data)
                .expect("Error writing byte data to file.");
            current_offset += data_length_in_bytes as u64;


            // RGB data
            if self.header.point_format == 2
                || self.header.point_format == 3
                || self.header.point_format == 5
                || self.header.point_format == 7
                || self.header.point_format == 8
            {
                let mut data_r = Vec::with_capacity(block_size * 2);
                let mut data_g = Vec::with_capacity(block_size * 2);
                let mut data_b = Vec::with_capacity(block_size * 2);
                for i in block_start..block_end {
                    data_r
                        .write_u16::<LittleEndian>(self.colour_data[i].red)
                        .expect("Error writing byte data.");
                    data_g
                        .write_u16::<LittleEndian>(self.colour_data[i].green)
                        .expect("Error writing byte data.");
                    data_b
                        .write_u16::<LittleEndian>(self.colour_data[i].blue)
                        .expect("Error writing byte data.");
                }
                // r
                let compressed = if compression_method == 0 {
                    // DEFLATE
                    compress_to_vec_zlib(&data_r, compression_level)
                } else if compression_method == 1 {
                    // brotli
                    brotli_compress(data_r.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                writer.write_u8(14u8).expect("Error writing byte data to file."); // Field code
                current_offset += 1;
                writer
                    .write_u64::<LittleEndian>(current_offset + 16)
                    .expect("Error writing byte data to file."); // FileOffset to data byte
                current_offset += 8;
                data_length_in_bytes = compressed.len() as u64;
                writer
                    .write_u64::<LittleEndian>(data_length_in_bytes)
                    .expect("Error writing byte data to file."); // ByteLength to data byte
                current_offset += 8;
                // byte alignment to words.
                // if data_length_in_bytes % 4 > 0 {
                //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
                //         compressed.push(0u8);
                //         current_offset += 1;
                //     }
                // }
                writer
                    .write_all(&compressed)
                    .expect("Error writing byte data to file.");
                current_offset += data_length_in_bytes as u64;


                // g
                let compressed = if compression_method == 0 {
                    // DEFLATE
                    compress_to_vec_zlib(&data_g, compression_level)
                } else if compression_method == 1 {
                    // brotli
                    brotli_compress(data_g.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                writer.write_u8(15u8).expect("Error writing byte data to file."); // Field code
                current_offset += 1;
                writer
                    .write_u64::<LittleEndian>(current_offset + 16)
                    .expect("Error writing byte data to file."); // FileOffset to data byte
                current_offset += 8;
                data_length_in_bytes = compressed.len() as u64;
                writer
                    .write_u64::<LittleEndian>(data_length_in_bytes)
                    .expect("Error writing byte data to file."); // ByteLength to data byte
                current_offset += 8;
                // byte alignment to words.
                // if data_length_in_bytes % 4 > 0 {
                //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
                //         compressed.push(0u8);
                //         current_offset += 1;
                //     }
                // }
                writer
                    .write_all(&compressed)
                    .expect("Error writing byte data to file.");
                current_offset += data_length_in_bytes as u64;


                // b
                let compressed = if compression_method == 0 {
                    // DEFLATE
                    compress_to_vec_zlib(&data_b, compression_level)
                } else if compression_method == 1 {
                    // brotli
                    brotli_compress(data_b.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                writer.write_u8(16u8).expect("Error writing byte data to file."); // Field code
                current_offset += 1;
                writer
                    .write_u64::<LittleEndian>(current_offset + 16)
                    .expect("Error writing byte data to file."); // FileOffset to data byte
                current_offset += 8;
                data_length_in_bytes = compressed.len() as u64;
                writer
                    .write_u64::<LittleEndian>(data_length_in_bytes)
                    .expect("Error writing byte data to file."); // ByteLength to data byte
                current_offset += 8;
                // byte alignment to words.
                // if data_length_in_bytes % 4 > 0 {
                //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
                //         compressed.push(0u8);
                //         current_offset += 1;
                //     }
                // }
                writer
                    .write_all(&compressed)
                    .expect("Error writing byte data to file.");
                current_offset += data_length_in_bytes as u64;
            }


            // NIR data
            if self.header.point_format == 8 {
                let mut data_nir = Vec::with_capacity(block_size * 2);
                for i in block_start..block_end {
                    data_nir
                        .write_u16::<LittleEndian>(self.colour_data[i].nir)
                        .expect("Error writing byte data.");
                }
                let compressed = if compression_method == 0 {
                    // DEFLATE
                    compress_to_vec_zlib(&data_nir, compression_level)
                } else if compression_method == 1 {
                    // brotli
                    brotli_compress(data_nir.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                writer.write_u8(17u8).expect("Error writing byte data to file."); // Field code
                current_offset += 1;
                writer
                    .write_u64::<LittleEndian>(current_offset + 16)
                    .expect("Error writing byte data to file."); // FileOffset to data byte
                current_offset += 8;
                data_length_in_bytes = compressed.len() as u64;
                writer
                    .write_u64::<LittleEndian>(data_length_in_bytes)
                    .expect("Error writing byte data to file."); // ByteLength to data byte
                current_offset += 8;
                // byte alignment to words.
                // if data_length_in_bytes % 4 > 0 {
                //     for _ in 0..(4 - (data_length_in_bytes % 4)) {
                //         compressed.push(0u8);
                //         current_offset += 1;
                //     }
                // }
                writer
                    .write_all(&compressed)
                    .expect("Error writing byte data to file.");
                current_offset += data_length_in_bytes as u64;
            }

            // Update the block start and end values
            block_start = block_end;
            block_end += block_size;
            if block_end > self.header.number_of_points as usize {
                block_end = self.header.number_of_points as usize;
            }
        }

        // let mut table_size = 4u64 + 10 * 20;
        // if self.use_point_intensity {
        //     table_size += 20u64;
        // }
        // if self.use_point_userdata {
        //     table_size += 20u64;
        // }
        // if self.header.point_format == 1 {
        //     table_size += 20u64;
        // } else if self.header.point_format == 2 {
        //     table_size += 3 * 20u64;
        // } else if self.header.point_format == 3 {
        //     table_size += 4 * 20u64;
        // }

        /*
        let compression_method = 1; // 0 = DEFLATE, 1 = brotli
        let compression_level = 5;
        // match compression_method {
        //     0 => println!("Compression method: DEFLATE-{}", compression_level),
        //     1 => println!("Compression method: Brotli-{}", compression_level),
        //     _ => println!("Compression method: not recognized"),
        // }
        let mut current_offset = self.header.offset_to_points as u64;
        let mut data_length_in_bytes: u64;
        let mut val: i32;
        let block_size = 50_000usize;
        let mut block_start = 0usize;
        let mut block_end = block_size;
        if block_end > self.header.number_of_points as usize {
            block_end = self.header.number_of_points as usize;
        }
        let mut flag = true;

        let mut change_byte_total = 0u64;
        let mut scanner_chan_total = 0u64;
        let mut ret_num_total = 0u64;
        let mut num_rets_total = 0u64;
        let mut class_total = 0u64;
        let mut time_total = 0u64;
        let mut scan_angle_total = 0u64;
        let mut flags_total = 0u64;
        let mut x_change_total = 0u64;
        let mut x_total = 0u64;
        let mut y_change_total = 0u64;
        let mut y_total = 0u64;
        let mut z_change_total = 0u64;
        let mut z_total = 0u64;
        let mut intensity_total = 0u64;
        let mut user_data_total = 0u64;
        let mut point_source_total = 0u64;
        let mut user_source_id_total = 0u64;
        let mut total_file_size = 0u64;

        // /*
        // let mut avg_comp = vec![0f64; 21];
        // let mut avg_comp_size = vec![0f64; 21];
        // let mut point_block = 0;
        // let mut total_bytes = 0usize;
        // let mut total_compressed_bytes = 0usize;

        // let mut histo_x = [0; 4];
        // let mut histo_y = [0; 4];
        // */
        while flag {
            let mut data_code = vec![];
            let mut byte_counts = vec![];
            let mut offsets = vec![];
            let mut output_data = vec![];

            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            let mut scanner_chan: usize;
            let mut ret_num_diff: i16;
            let mut val_u8: u8;

            // Change byte
            // let mut dt = Vec::with_capacity(block_size);
            // let mut prev_times = [0i64, 0i64, 0i64, 0i64];
            // let mut time_val: i64;
            // for i in block_start..block_end {
            //     scanner_chan = self.point_data[i].scanner_channel() as usize;
            //     if self.gps_data.len() > 0 {
            //         time_val = i64::from_le_bytes(self.gps_data[i].to_le_bytes());
            //         dt.push(time_val - prev_times[scanner_chan]);
            //         prev_times[scanner_chan] = time_val;
            //     }
            // }

            let mut change_data = Vec::with_capacity(block_size);
            change_data.push(0u8);
            // val_u8 = 0b0111_1111u8;
            for i in block_start+1..block_end {
                val_u8 = 0;
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].scanner_channel() != self.point_data[i-1].scanner_channel() {
                    val_u8 = val_u8 | 0b0000_0001u8;
                }

                if self.gps_data.len() > 0 && self.gps_data[i] != self.gps_data[scanner_chan_index[scanner_chan]] {
                // if self.gps_data.len() > 0 && dt[i - block_start] != dt[scanner_chan_index[scanner_chan] - block_start] {
                    val_u8 = val_u8 | 0b0000_0010u8;
                }

                ret_num_diff = self.point_data[i].return_number() as i16 - self.point_data[scanner_chan_index[scanner_chan]].return_number() as i16;
                if ret_num_diff == 1 {
                    val_u8 = val_u8 | 0b0000_0100u8;
                } else if ret_num_diff == -1 {
                    val_u8 = val_u8 | 0b0000_1000u8;
                } else if ret_num_diff != 0 {
                    val_u8 = val_u8 | 0b0000_1100u8;
                }

                if self.point_data[i].number_of_returns() != self.point_data[scanner_chan_index[scanner_chan]].number_of_returns() {
                    val_u8 = val_u8 | 0b0001_0000u8;
                }

                if self.point_data[i].classification() != self.point_data[scanner_chan_index[scanner_chan]].classification() {
                    val_u8 = val_u8 | 0b0010_0000u8;
                }

                if self.point_data[i].scan_angle != self.point_data[scanner_chan_index[scanner_chan]].scan_angle {
                    val_u8 = val_u8 | 0b0100_0000u8;
                }

                if self.point_data[i].intensity > 255 {
                    val_u8 = val_u8 | 0b1000_0000u8;
                }

                change_data.push(val_u8);

                scanner_chan_index[scanner_chan] = i;
            }
            // total_bytes += change_data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&change_data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(change_data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            change_byte_total += data_length_in_bytes;

            data_code.push(13u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }

            // Scanner channel
            let mut data = Vec::with_capacity(block_size);
            val_u8 = self.point_data[0].scanner_channel();
            let num_bits = 2;
            let mut num_bits_in_byte = num_bits;
            for i in block_start+1..block_end {
                if self.point_data[i].scanner_channel() != self.point_data[i-1].scanner_channel() {
                    val_u8 = val_u8 | (self.point_data[i].scanner_channel() << num_bits_in_byte);
                    num_bits_in_byte += num_bits;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8);
                        val_u8 = 0;
                        num_bits_in_byte = 0;
                    }
                }
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            scanner_chan_total += data_length_in_bytes;

            data_code.push(14u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // Return number
            let mut data = Vec::with_capacity(block_size);
            val_u8 = self.point_data[0].return_number();
            let num_bits = 4;
            let mut num_bits_in_byte = num_bits;
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start+1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                ret_num_diff = self.point_data[i].return_number() as i16 - self.point_data[scanner_chan_index[scanner_chan]].return_number() as i16;
                // if self.point_data[i].return_number() != self.point_data[scanner_chan_index[scanner_chan]].return_number() {
                if ret_num_diff.abs() > 1 {
                    val_u8 = val_u8 | (self.point_data[i].return_number() << num_bits_in_byte);
                    num_bits_in_byte += num_bits;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8);
                        val_u8 = 0;
                        num_bits_in_byte = 0;
                    }
                }
                scanner_chan_index[scanner_chan] = i;
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            ret_num_total += data_length_in_bytes;

            data_code.push(15u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // Number of returns
            let mut data = Vec::with_capacity(block_size);
            val_u8 = self.point_data[0].number_of_returns();
            let num_bits = 4;
            let mut num_bits_in_byte = num_bits;
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start+1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].number_of_returns() != self.point_data[scanner_chan_index[scanner_chan]].number_of_returns() {
                    val_u8 = val_u8 | (self.point_data[i].number_of_returns() << num_bits_in_byte);
                    num_bits_in_byte += num_bits;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8);
                        val_u8 = 0;
                        num_bits_in_byte = 0;
                    }
                }
                scanner_chan_index[scanner_chan] = i;
            }
            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            num_rets_total += data_length_in_bytes;

            data_code.push(16u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // Classification byte
            let mut data = Vec::with_capacity(block_size);
            data.push(self.point_data[0].classification());
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start+1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].classification() != self.point_data[scanner_chan_index[scanner_chan]].classification() {
                    data.push(self.point_data[i].classification());
                    // scanner_chan_index[scanner_chan] = i;
                }
                scanner_chan_index[scanner_chan] = i;
            }
            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            class_total += data_length_in_bytes;

            data_code.push(5u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // GPS time
            // let mut delta_values = Vec::with_capacity(block_size);
            // let mut prev_times = [0i64, 0i64, 0i64, 0i64];
            // let mut time_val: i64;
            // for i in block_start..block_end {
            //     scanner_chan = self.point_data[i].scanner_channel() as usize;
            //     if self.gps_data.len() > 0 {
            //         time_val = i64::from_le_bytes(self.gps_data[i].to_le_bytes());
            //         delta_values.push(time_val - prev_times[scanner_chan]);
            //         prev_times[scanner_chan] = time_val;
            //     }
            // }
            // let mut time_diff: i64;

            // let mut cntx: usize;
            // let mut prev_index = [[block_start; 16], [block_start; 16], [block_start; 16], [block_start; 16]];
            let mut data = Vec::with_capacity(block_size * 8);
            if self.gps_data.len() > 0 {
                data.write_f64::<LittleEndian>(self.gps_data[0]).expect("Error writing byte data.");
            } else {
                data.write_f64::<LittleEndian>(0f64).expect("Error writing byte data.");
            };
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start+1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.gps_data.len() > 0 && self.gps_data[i] != self.gps_data[scanner_chan_index[scanner_chan]] {
                    data.write_f64::<LittleEndian>(self.gps_data[i] - self.gps_data[scanner_chan_index[scanner_chan]]).expect("Error writing byte data.");
                }

                // if self.gps_data.len() > 0 {
                //     cntx = self.get_context(i);
                //     if dt[i - block_start] != dt[prev_index[scanner_chan][cntx]-block_start] {
                //         data.write_i64::<LittleEndian>(dt[i - block_start]).expect("Error writing byte data.");
                //     }
                //     prev_index[scanner_chan][cntx] = i;
                // }

                // if self.gps_data.len() > 0 && delta_values[i - block_start] != 0i64 {
                //     cntx = self.get_context(i);
                //     time_diff = delta_values[i - block_start] - delta_values[scanner_chan_index[scanner_chan] - block_start];
                //     if i >= 500_000 && i < 500_050 {
                //         println!("{} {}", time_diff, self.get_short_filename());
                //     }
                //     data.write_i64::<LittleEndian>(time_diff).expect("Error writing byte data.");
                //     // time_diff = self.gps_data[i] - self.gps_data[scanner_chan_index[scanner_chan]];
                //     // if time_diff != 0f64 && time_diff.abs() != time_interval && time_diff.abs() != two_time_interval {
                //     //     data.write_f64::<LittleEndian>(time_diff).expect("Error writing byte data.");
                //     // }
                // }

                scanner_chan_index[scanner_chan] = i;
            }
            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            time_total += data_length_in_bytes;

            data_code.push(9u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // Scan angle
            let mut data = Vec::with_capacity(block_size * 2);
            data.write_i16::<LittleEndian>(self.point_data[0].scan_angle).expect("Error writing byte data.");
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start+1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                if self.point_data[i].scan_angle != self.point_data[scanner_chan_index[scanner_chan]].scan_angle {
                    data.write_i16::<LittleEndian>(self.point_data[i].scan_angle).expect("Error writing byte data.");
                    // data.write_i16::<LittleEndian>(self.point_data[i].scan_angle - self.point_data[scanner_chan_index[scanner_chan]].scan_angle).expect("Error writing byte data.");
                    // scanner_chan_index[scanner_chan] = i;
                }
                scanner_chan_index[scanner_chan] = i;
            }
            // total_bytes += data.len() / 2;
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            scan_angle_total += data_length_in_bytes;

            data_code.push(6u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // Flags
            let mut data = Vec::with_capacity(block_size);
            val_u8 = 0;
            if self.point_data[0].synthetic() {
                val_u8 = val_u8 | 0b0000_0001u8;
            }
            if self.point_data[0].keypoint() {
                val_u8 = val_u8 | 0b0000_0010u8;
            }
            if self.point_data[0].withheld() {
                val_u8 = val_u8 | 0b0000_0100u8;
            }
            if self.point_data[0].overlap() {
                val_u8 = val_u8 | 0b0000_1000u8;
            }
            if self.point_data[0].scan_direction_flag() {
                val_u8 = val_u8 | 0b0001_0000u8;
            }
            if self.point_data[0].edge_of_flightline_flag() {
                val_u8 = val_u8 | 0b0010_0000u8;
            }
            data.push(val_u8);
            let mut scanner_chan_index = [block_start, block_start, block_start, block_start];
            for i in block_start+1..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                val_u8 = 0;
                if self.point_data[i].synthetic() {
                    val_u8 = val_u8 | 0b0000_0001u8;
                }
                if self.point_data[i].keypoint() {
                    val_u8 = val_u8 | 0b0000_0010u8;
                }
                if self.point_data[i].withheld() {
                    val_u8 = val_u8 | 0b0000_0100u8;
                }
                if self.point_data[i].overlap() {
                    val_u8 = val_u8 | 0b0000_1000u8;
                }
                if self.point_data[i].scan_direction_flag() {
                    val_u8 = val_u8 | 0b0001_0000u8;
                }
                if self.point_data[i].edge_of_flightline_flag() {
                    val_u8 = val_u8 | 0b0010_0000u8;
                }
                data.push(val_u8);
                scanner_chan_index[scanner_chan] = i;
            }

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            flags_total += data_length_in_bytes;

            data_code.push(17u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // x
            let mut delta_values = Vec::with_capacity(block_size);
            let mut prev_vals = [0i32, 0i32, 0i32, 0i32];
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                delta_values.push(val - prev_vals[scanner_chan]);
                prev_vals[scanner_chan] = val;
            }

            let mut data = Vec::with_capacity(block_size / 4 + 1);
            let mut data2 = Vec::with_capacity(block_size * 4);

            let mut prev_index = [[block_start; 16], [block_start; 16], [block_start; 16], [block_start; 16]];
            let mut cntx: usize;
            let mut val: i32;
            let mut val2: i32;
            let mut tag = 0u8;
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                val = delta_values[i-block_start];
                cntx = self.get_context(i);
                val2 = val - delta_values[prev_index[scanner_chan][cntx]-block_start];

                if i > block_start {
                    tag = if val2.abs() <= 6 { // half-byte
                        // histo_x[0] += 1;
                        (val2 + 6) as u8
                    } else if val2 >= -128 && val2 <= 127 { // 1 byte
                        // histo_x[1] += 1;
                        data2.write_i8(val2 as i8).expect("Error writing byte data.");
                        13u8
                    } else if val2 >= -32768 && val2 <= 32767 { // 2 bytes
                        // histo_x[2] += 1;
                        data2.write_i16::<LittleEndian>(val2 as i16).expect("Error writing byte data.");
                        14u8
                    } else { // 4 bytes
                        // histo_x[3] += 1;
                        data2.write_i32::<LittleEndian>(val2).expect("Error writing byte data.");
                        15u8
                    };

                    val_u8 = val_u8 | (tag << num_bits_in_byte);
                    num_bits_in_byte += 4;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8); // flush it
                        val_u8 = 0; // reset it
                        num_bits_in_byte = 0; // reset it
                    }
                } else {
                    val_u8 = 15u8;
                    num_bits_in_byte = 4;
                    data2.write_i32::<LittleEndian>(val).expect("Error writing byte data.");
                }

                prev_index[scanner_chan][cntx] = i;
            }

            if num_bits_in_byte > 0 {
                data.push(val_u8); // flush it
            }

            // data.append(&mut data2);

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            x_change_total += data_length_in_bytes;

            data_code.push(18u32); // 18
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }

            // total_bytes += data2.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data2, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data2.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            x_total += data_length_in_bytes;

            data_code.push(0u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // y
            let mut delta_values = Vec::with_capacity(block_size);
            let mut prev_vals = [0i32, 0i32, 0i32, 0i32];
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                delta_values.push(val - prev_vals[scanner_chan]);
                prev_vals[scanner_chan] = val;
            }

            let mut data = Vec::with_capacity(block_size / 4 + 1);
            let mut data2 = Vec::with_capacity(block_size * 4);

            let mut prev_index = [[block_start; 16], [block_start; 16], [block_start; 16], [block_start; 16]];
            let mut cntx: usize;
            let mut val: i32;
            let mut val2: i32;
            let mut tag = 0u8;
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                val = delta_values[i-block_start];
                cntx = self.get_context(i);
                val2 = val - delta_values[prev_index[scanner_chan][cntx]-block_start];

                if i > block_start {
                    tag = if val2.abs() <= 6 { // half-byte
                        // histo_y[0] += 1;
                        (val2 + 6) as u8
                    } else if val2 >= -128 && val2 <= 127 { // 1 byte
                        // histo_y[1] += 1;
                        data2.write_i8(val2 as i8).expect("Error writing byte data.");
                        13u8
                    } else if val2 >= -32768 && val2 <= 32767 { // 2 bytes
                        // histo_y[2] += 1;
                        data2.write_i16::<LittleEndian>(val2 as i16).expect("Error writing byte data.");
                        14u8
                    } else { // 4 bytes
                        // histo_y[3] += 1;
                        data2.write_i32::<LittleEndian>(val2).expect("Error writing byte data.");
                        15u8
                    };

                    val_u8 = val_u8 | (tag << num_bits_in_byte);
                    num_bits_in_byte += 4;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8); // flush it
                        val_u8 = 0; // reset it
                        num_bits_in_byte = 0; // reset it
                    }
                } else {
                    val_u8 = 15u8;
                    num_bits_in_byte = 4;
                    data2.write_i32::<LittleEndian>(val).expect("Error writing byte data.");
                }

                prev_index[scanner_chan][cntx] = i;
            }

            if num_bits_in_byte > 0 {
                data.push(val_u8); // flush it
            }

            // data.append(&mut data2);

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            y_change_total += data_length_in_bytes;

            data_code.push(19u32); // 19
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }

            // total_bytes += data2.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data2, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data2.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            y_total += data_length_in_bytes;

            data_code.push(1u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // z
            let mut data = Vec::with_capacity(block_size / 4 + 1);
            let mut data2 = Vec::with_capacity(block_size * 4);
            let mut prev_late_vals = [0i32, 0i32, 0i32, 0i32];
            let mut prev_early_vals = [0i32, 0i32, 0i32, 0i32];
            let mut prev_val = 0i32;
            let mut val: i32;
            let mut tag = 0u8;
            for i in block_start..block_end {
                scanner_chan = self.point_data[i].scanner_channel() as usize;
                prev_val = if self.point_data[i].is_late_return() {
                    prev_late_vals[scanner_chan]
                } else {
                    prev_early_vals[scanner_chan]
                };
                val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                val2 = val - prev_val;

                if i > block_start {
                    tag = if val2.abs() <= 6 { // half-byte
                        (val2 + 6) as u8
                    } else if val2 >= -128 && val2 <= 127 { // 1 byte
                        data2.write_i8(val2 as i8).expect("Error writing byte data.");
                        13u8 // tag for a single signed byte
                    } else if val2 >= -32768 && val2 <= 32767 { // 2 bytes
                        data2.write_i16::<LittleEndian>(val2 as i16).expect("Error writing byte data.");
                        14u8 // tag for a 2-byte signed int
                    } else { // 4 bytes
                        data2.write_i32::<LittleEndian>(val2).expect("Error writing byte data.");
                        15u8 // tag for a 4-byte signed int
                    };

                    val_u8 = val_u8 | (tag << num_bits_in_byte);
                    num_bits_in_byte += 4;
                    if num_bits_in_byte == 8 {
                        data.push(val_u8); // flush it
                        val_u8 = 0; // reset it
                        num_bits_in_byte = 0; // reset it
                    }
                } else {
                    val_u8 = 15u8;
                    num_bits_in_byte = 4;
                    data2.write_i32::<LittleEndian>(val2).expect("Error writing byte data.");
                }

                if self.point_data[i].is_late_return() {
                    prev_late_vals[scanner_chan] = val;
                } else {
                    prev_early_vals[scanner_chan] = val;
                }
            }

            if num_bits_in_byte > 0 {
                data.push(val_u8); // flush it
            }

            // data.append(&mut data2);

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            z_change_total += data_length_in_bytes;

            data_code.push(20u32); // 20
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }

            // total_bytes += data2.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data2, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data2.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            z_total += data_length_in_bytes;

            data_code.push(2u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }


            // intensity
            if self.use_point_intensity {
                let mut data = Vec::with_capacity(block_size * 2);
                let mut val = 0u16;
                for i in block_start..block_end {
                    val = self.point_data[i].intensity;
                    if val < 256 {
                        data.write_u8(val as u8).expect("Error writing byte data.");
                    } else {
                        data.write_u16::<LittleEndian>(val).expect("Error writing byte data.");
                    }
                }
                // total_bytes += data.len();
                let mut compressed = if compression_method == 0 { // DEFLATE
                    compress_to_vec_zlib(&data, compression_level)
                } else if compression_method == 1 { // brotli
                    brotli_compress(data.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                data_length_in_bytes = compressed.len() as u64;
                output_data.append(&mut compressed);
                intensity_total += data_length_in_bytes;

                data_code.push(3u32);
                byte_counts.push(data_length_in_bytes);
                offsets.push(current_offset);
                current_offset += data_length_in_bytes;
                // byte alignment to words.
                if data_length_in_bytes % 4 > 0 {
                    for _ in 0..(4 - (data_length_in_bytes % 4)) {
                        output_data.push(0u8);
                        current_offset += 1;
                    }
                }
            }


            // User data and point source ID change byte
            let mut data = Vec::with_capacity(block_size);
            val_u8 = 0b0000_0011;
            let num_bits = 2;
            let mut num_bits_in_byte = num_bits;
            let mut b: u8;
            for i in block_start+1..block_end {
                b = if self.point_data[i].user_data != self.point_data[i-1].user_data &&
                    self.point_data[i].point_source_id != self.point_data[i-1].point_source_id {
                        0b0000_0011
                } else if self.point_data[i].user_data != self.point_data[i-1].user_data &&
                    self.point_data[i].point_source_id == self.point_data[i-1].point_source_id {
                        0b0000_0001
                } else if self.point_data[i].user_data == self.point_data[i-1].user_data &&
                    self.point_data[i].point_source_id != self.point_data[i-1].point_source_id {
                        0b0000_0010
                } else {
                        0b0000_0000
                };
                val_u8 = val_u8 | (b << num_bits_in_byte);
                num_bits_in_byte += num_bits;
                if num_bits_in_byte == 8 {
                    data.push(val_u8);
                    val_u8 = 0;
                    num_bits_in_byte = 0;
                }
            }

            for i in block_start+1..block_end {
                b = if self.point_data[i].user_data != self.point_data[i-1].user_data &&
                    self.point_data[i].point_source_id != self.point_data[i-1].point_source_id {
                        0b0000_0011
                } else if self.point_data[i].user_data != self.point_data[i-1].user_data &&
                    self.point_data[i].point_source_id == self.point_data[i-1].point_source_id {
                        0b0000_0001
                } else if self.point_data[i].user_data == self.point_data[i-1].user_data &&
                    self.point_data[i].point_source_id != self.point_data[i-1].point_source_id {
                        0b0000_0010
                } else {
                        0b0000_0000
                };
                val_u8 = val_u8 | (b << num_bits_in_byte);
                num_bits_in_byte += num_bits;
                if num_bits_in_byte == 8 {
                    data.push(val_u8);
                    val_u8 = 0;
                    num_bits_in_byte = 0;
                }
            }

            if num_bits_in_byte > 0 {
                data.push(val_u8);
            }

            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            user_source_id_total += data_length_in_bytes;

            data_code.push(21u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }

            // user data
            if self.use_point_userdata {
                let mut data = Vec::with_capacity(block_size);
                let mut prev_val = 0u8;
                for i in block_start..block_end {
                    if self.point_data[i].user_data != prev_val {
                        data.push(self.point_data[i].user_data);
                        prev_val = self.point_data[i].user_data;
                    }
                }
                // total_bytes += data.len();
                let mut compressed = if compression_method == 0 { // DEFLATE
                    compress_to_vec_zlib(&data, compression_level)
                } else if compression_method == 1 { // brotli
                    brotli_compress(data.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                data_length_in_bytes = compressed.len() as u64;
                output_data.append(&mut compressed);
                user_data_total += data_length_in_bytes;

                data_code.push(7u32);
                byte_counts.push(data_length_in_bytes);
                offsets.push(current_offset);
                current_offset += data_length_in_bytes;
                // byte alignment to words.
                if data_length_in_bytes % 4 > 0 {
                    for _ in 0..(4 - (data_length_in_bytes % 4)) {
                        output_data.push(0u8);
                        current_offset += 1;
                    }
                }
            }

            // point_source_id
            let mut data = Vec::with_capacity(block_size * 2);
            let mut prev_val = 0u16;
            for i in block_start..block_end {
                if self.point_data[i].point_source_id != prev_val {
                    data.write_u16::<LittleEndian>(self.point_data[i].point_source_id)
                        .expect("Error writing byte data.");
                    prev_val = self.point_data[i].point_source_id;
                }
            }
            // total_bytes += data.len();
            let mut compressed = if compression_method == 0 { // DEFLATE
                compress_to_vec_zlib(&data, compression_level)
            } else if compression_method == 1 { // brotli
                brotli_compress(data.as_slice(), compression_level)
            } else {
                panic!("Unrecognized compression method.");
            };
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            point_source_total += data_length_in_bytes;

            data_code.push(8u32);
            byte_counts.push(data_length_in_bytes);
            offsets.push(current_offset);
            current_offset += data_length_in_bytes;
            // byte alignment to words.
            if data_length_in_bytes % 4 > 0 {
                for _ in 0..(4 - (data_length_in_bytes % 4)) {
                    output_data.push(0u8);
                    current_offset += 1;
                }
            }

            if self.colour_data.len() > 0 {
                // colour_data
                let mut data_r = Vec::with_capacity(block_size * 2);
                let mut data_g = Vec::with_capacity(block_size * 2);
                let mut data_b = Vec::with_capacity(block_size * 2);
                for i in block_start..block_end {
                    data_r
                        .write_u16::<LittleEndian>(self.colour_data[i].red)
                        .expect("Error writing byte data.");
                    data_g
                        .write_u16::<LittleEndian>(self.colour_data[i].green)
                        .expect("Error writing byte data.");
                    data_b
                        .write_u16::<LittleEndian>(self.colour_data[i].blue)
                        .expect("Error writing byte data.");
                }
                // r
                // total_bytes += data_r.len();
                let mut compressed = if compression_method == 0 { // DEFLATE
                    compress_to_vec_zlib(&data_r, compression_level)
                } else if compression_method == 1 { // brotli
                    brotli_compress(data_r.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                data_length_in_bytes = compressed.len() as u64;
                output_data.append(&mut compressed);
                data_code.push(10u32);
                byte_counts.push(data_length_in_bytes);
                offsets.push(current_offset);
                current_offset += data_length_in_bytes;
                // byte alignment to words.
                if data_length_in_bytes % 4 > 0 {
                    for _ in 0..(4 - (data_length_in_bytes % 4)) {
                        output_data.push(0u8);
                        current_offset += 1;
                    }
                }

                // g
                // total_bytes += data_g.len();
                let mut compressed = if compression_method == 0 { // DEFLATE
                    compress_to_vec_zlib(&data_g, compression_level)
                } else if compression_method == 1 { // brotli
                    brotli_compress(data_g.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                data_length_in_bytes = compressed.len() as u64;
                output_data.append(&mut compressed);
                data_code.push(11u32);
                byte_counts.push(data_length_in_bytes);
                offsets.push(current_offset);
                current_offset += data_length_in_bytes;
                // byte alignment to words.
                if data_length_in_bytes % 4 > 0 {
                    for _ in 0..(4 - (data_length_in_bytes % 4)) {
                        output_data.push(0u8);
                        current_offset += 1;
                    }
                }

                // b
                // total_bytes += data_b.len();
                let mut compressed = if compression_method == 0 { // DEFLATE
                    compress_to_vec_zlib(&data_b, compression_level)
                } else if compression_method == 1 { // brotli
                    brotli_compress(data_b.as_slice(), compression_level)
                } else {
                    panic!("Unrecognized compression method.");
                };
                data_length_in_bytes = compressed.len() as u64;
                output_data.append(&mut compressed);
                data_code.push(12u32);
                byte_counts.push(data_length_in_bytes);
                offsets.push(current_offset);
                current_offset += data_length_in_bytes;
                // byte alignment to words.
                if data_length_in_bytes % 4 > 0 {
                    for _ in 0..(4 - (data_length_in_bytes % 4)) {
                        output_data.push(0u8);
                        current_offset += 1;
                    }
                }
                drop(data_r);
                drop(data_g);
                drop(data_b);
            }


            // Now write the data. Start with the point data table
            let table_length = 4u64 + data_code.len() as u64 * 20u64;
            // writer
            //     .write_u16::<LittleEndian>(data_code.len() as u16)
            //     .expect("Error writing byte data to file.");
            writer
                .write_u8(data_code.len() as u8)
                .expect("Error writing byte data to file.");
            // let compression_method = 0u8; // DEFLATE (ZLIB)
            // let compression_method = 1u8;
            writer
                .write_u8(compression_method)
                .expect("Error writing byte data to file.");
            writer
                .write_u8(1u8)
                .expect("Error writing byte data to file."); // zlidar major version number
            writer
                .write_u8(1u8)
                .expect("Error writing byte data to file."); // zlidar minor version number

            for i in 0..data_code.len() {
                let mut data = Vec::with_capacity(20);
                data.write_u32::<LittleEndian>(data_code[i])
                    .expect("Error writing byte data.");
                data.write_u64::<LittleEndian>(offsets[i] + table_length)
                    .expect("Error writing byte data.");
                data.write_u64::<LittleEndian>(byte_counts[i])
                    .expect("Error writing byte data.");
                writer
                    .write_all(&data)
                    .expect("Error writing byte data to file.");

                // /*
                // let byte_size = if data_code[i] < 3 {
                //     4usize
                // } else if data_code[i] == 3 {
                //     2usize
                // } else if data_code[i] < 8 {
                //     1usize
                // } else if data_code[i] == 8 {
                //     2usize
                // } else {
                //     8usize
                // };
                // avg_comp[data_code[i] as usize] +=
                //     100f64 * (byte_counts[i] as f64 / (block_size * byte_size) as f64);
                // avg_comp_size[data_code[i] as usize] += byte_counts[i] as f64
                // */
            }

            // total_compressed_bytes += output_data.len();
            total_file_size += output_data.len() as u64;

            writer
                .write_all(&output_data)
                .expect("Error writing byte data to file.");

            current_offset += table_length;

            // point_block += 1; // comment out
            block_start = block_end;
            if block_start >= self.header.number_of_points as usize {
                flag = false;
            }
            block_end += block_size;
            if block_end > self.header.number_of_points as usize {
                block_end = self.header.number_of_points as usize;
            }
        }


        println!("change_byte: {}, {:.2}", change_byte_total, 100.0 * change_byte_total as f64 / total_file_size as f64);
        println!("scanner_chan: {}, {:.2}", scanner_chan_total, 100.0 * scanner_chan_total as f64 / total_file_size as f64);
        println!("ret_num: {}, {:.2}", ret_num_total, 100.0 * ret_num_total as f64 / total_file_size as f64);
        println!("num_rets: {}, {:.2}", num_rets_total, 100.0 * num_rets_total as f64 / total_file_size as f64);
        println!("classification: {}, {:.2}", class_total, 100.0 * class_total as f64 / total_file_size as f64);
        println!("time: {}, {:.2}", time_total, 100.0 * time_total as f64 / total_file_size as f64);
        println!("scan_angle: {}, {:.2}", scan_angle_total, 100.0 * scan_angle_total as f64 / total_file_size as f64);
        println!("flags: {}, {:.2}", flags_total, 100.0 * flags_total as f64 / total_file_size as f64);
        println!("x_change: {}, {:.2}", x_change_total, 100.0 * x_change_total as f64 / total_file_size as f64);
        println!("x: {}, {:.2}", x_total, 100.0 * x_total as f64 / total_file_size as f64);
        println!("y_change: {}, {:.2}", y_change_total, 100.0 * y_change_total as f64 / total_file_size as f64);
        println!("y: {}, {:.2}", y_total, 100.0 * y_total as f64 / total_file_size as f64);
        println!("z_change: {}, {:.2}", z_change_total, 100.0 * z_change_total as f64 / total_file_size as f64);
        println!("z: {}, {:.2}", z_total, 100.0 * z_total as f64 / total_file_size as f64);
        println!("intensity: {}, {:.2}", intensity_total, 100.0 * intensity_total as f64 / total_file_size as f64);
        println!("user/source_id: {}, {:.2}", user_source_id_total, 100.0 * user_source_id_total as f64 / total_file_size as f64);
        println!("user_data: {}, {:.2}", user_data_total, 100.0 * user_data_total as f64 / total_file_size as f64);
        println!("point_source_id: {}, {:.2}", point_source_total, 100.0 * point_source_total as f64 / total_file_size as f64);
        let total = change_byte_total + scanner_chan_total + ret_num_total + num_rets_total + class_total +
            time_total + scan_angle_total + flags_total + x_change_total + x_total + y_change_total + y_total +
            z_change_total + z_total + intensity_total + user_data_total + point_source_total;
        let header_total = total_file_size - total;
        println!("headers: {}, {:.2}", header_total, 100.0 * header_total as f64 / total_file_size as f64);
        println!("Total: {}, {:.5}", total_file_size, self.header.number_of_points as f64 / total_file_size as f64);

        */

        /*
        let n_points = self.header.number_of_points as f64;
        let sum_x = histo_x[0]/2 + histo_x[1] + 2*histo_x[2] + 4*histo_x[3];
        println!("0.5: {:.1}, 1: {:.1}, 2: {:.1}, 4: {:.1}, bytes: {} ({:.1})",
            histo_x[0] as f64 * 100.0 / n_points,
            histo_x[1] as f64 * 100.0 / n_points,
            histo_x[2] as f64 * 100.0 / n_points,
            histo_x[3] as f64 * 100.0 / n_points,
            sum_x,
            100.0*sum_x as f64 / (n_points*4.0)
        );

        let sum_y = histo_y[0]/2 + histo_y[1] + 2*histo_y[2] + 4*histo_y[3];
        println!("0.5: {:.1}, 1: {:.1}, 2: {:.1}, 4: {:.1}, bytes: {} ({:.1})",
            histo_y[0] as f64 * 100.0 / n_points,
            histo_y[1] as f64 * 100.0 / n_points,
            histo_y[2] as f64 * 100.0 / n_points,
            histo_y[3] as f64 * 100.0 / n_points,
            sum_y,
            100.0*sum_y as f64 / (n_points*4.0)
        );

        println!("Average compressed size:");
        for a in 0..avg_comp.len() {
            avg_comp[a] /=  point_block as f64;
            avg_comp_size[a] /= point_block as f64;
            match a {
                0usize => println!("x: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                18usize => {}, // println!("x byte: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                1usize => println!("y: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                19usize => {}, // println!("y byte: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                2usize => println!("z: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                20usize => {}, // println!("z byte: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                3usize => println!("intensity: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                4usize => {}, // println!("point byte: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                5usize => println!("classification: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                6usize => println!("scan angle: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                7usize => println!("user data: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                8usize => println!("source id: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                9usize => println!("GPS time: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                13usize => println!("change byte: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                14usize => println!("scan chan: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                15usize => println!("ret num: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                16usize => println!("num rets: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                17usize => println!("flags: {:.1}, ({:.2})", avg_comp_size[a], avg_comp[a]),
                _ => {}, // do nothing
            }
        }
        println!("Overall: {} ({:.2} / {:.2})", total_compressed_bytes, 100.0 * total_compressed_bytes as f64 / total_bytes as f64, total_bytes as f64 / total_compressed_bytes as f64);
        */

        let _ = writer.flush();

        Ok(())
    }

    pub fn get_vlr_data_as_string(&self) -> String {
        let mut s = "".to_string();
        let mut i: usize = 1;
        for vlr in &self.vlr_data {
            s = s + &format!("\nVLR {}:\n{}", i, vlr);
            i += 1;
        }
        return s;
    }

    fn get_context(&self, i: usize) -> usize {
        let cntx1 = if self.point_data[i].is_only_return() {
            0
        } else if self.point_data[i].is_last_return() {
            1
        } else if self.point_data[i].is_intermediate_return() {
            2
        } else {
            // first
            3
        };

        let cntx2 = if i == 0 || self.point_data[i - 1].is_only_return() {
            0
        } else if self.point_data[i - 1].is_last_return() {
            1
        } else if self.point_data[i - 1].is_intermediate_return() {
            2
        } else {
            // first
            3
        };

        cntx1 * 4 + cntx2
    }
}

fn brotli_compress(input: &[u8], level: u8) -> Vec<u8> {
    let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, level as u32, 22);
    writer.write_all(input).unwrap();
    writer.into_inner()
}

fn brotli_decompress(input: &[u8]) -> Vec<u8> {
    // let mut writer = brotli::DecompressorWriter::new(Vec::new(), 4096);
    // writer.write_all(input).unwrap();
    // let output = match writer.into_inner() {
    //     Ok(v) => v,
    //     Err(_) => panic!("Brotli error while decoding data."),
    // };
    // output

    // let mut reader = brotli::Decompressor::new(
    //     input,
    //     4096, // buffer size
    // );
    // let mut buf = [0u8; 4096];
    // let mut output = vec![];
    // loop {
    //     match reader.read(&mut buf[..]) {
    //         Err(e) => {
    //             if let std::io::ErrorKind::Interrupted = e.kind() {
    //                 continue;
    //             }
    //             panic!(e);
    //         }
    //         Ok(size) => {
    //             println!("I'm here");
    //             if size == 0 {
    //                 break;
    //             }
    //             for a in 0..size {
    //                 output.push(buf[a]);
    //             }
    //         }
    //     }
    // } 
    // output 
    
    if input.len() == 0 {
        panic!("Zero-length input for Brotli decompression");
    }
    let mut output = Vec::new();
    {
        let mut writer = brotli::DecompressorWriter::new(&mut output, 4096);
        let _ = writer.write(&input);
    }
    output
}

impl fmt::Display for LasFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            format!("File Name: {}\n{}", self.file_name, &self.header)
        )
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct GlobalEncodingField {
    pub value: u16,
}

impl GlobalEncodingField {
    /// Returns the GPS Type within the Global Encoding bit field.
    pub fn gps_time(&self) -> GpsTimeType {
        if (self.value & 1u16) == 1u16 {
            GpsTimeType::SatelliteGpsTime
        } else {
            GpsTimeType::GpsWeekTime
        }
    }

    /// Returns a boolean indicating whether waveform packet data is stored
    /// internally to the file.
    pub fn waveform_data_internal(&self) -> bool {
        //((self.value >> 1_u16) & 1_u16) == 1_u16
        (self.value & 0b0000_0010u16) == 0b0000_0010u16
    }

    /// Returns a boolean indicating whether waveform packet data is stored
    /// externally to the file.
    pub fn waveform_data_external(&self) -> bool {
        //((self.value >> 2_u16) & 1_u16) == 1_u16
        (self.value & 0b0000_0100u16) == 0b0000_0100u16
    }

    /// Returns a boolean indicating whether the return numbers have been
    /// generated synthetically.
    pub fn return_data_synthetic(&self) -> bool {
        //((self.value >> 3_u16) & 1_u16) == 1_u16
        (self.value & 0b0000_1000u16) == 0b0000_1000u16
    }

    /// Returns the coordinate reference system method used within the file.
    pub fn coordinate_reference_system_method(&self) -> CoordinateReferenceSystem {
        if (self.value & 0b0001_0000u16) == 0b0001_0000u16 {
            CoordinateReferenceSystem::WellKnownText
        } else {
            CoordinateReferenceSystem::GeoTiff
        }
    }
}

impl fmt::Display for GlobalEncodingField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "    GPS time={:?},
    Waveform data internal={},
    Waveform data external={},
    Return data synthetic={},
    CRS method={:?}",
            self.gps_time(),
            self.waveform_data_internal(),
            self.waveform_data_external(),
            self.return_data_synthetic(),
            self.coordinate_reference_system_method()
        )
    }
}

#[derive(Debug)]
pub enum GpsTimeType {
    GpsWeekTime,
    SatelliteGpsTime,
}

#[derive(Debug)]
pub enum CoordinateReferenceSystem {
    WellKnownText,
    GeoTiff,
}

#[derive(Clone, Copy, Debug)]
pub enum LidarPointRecord {
    PointRecord0 {
        point_data: PointData,
    },
    PointRecord1 {
        point_data: PointData,
        gps_data: f64,
    },
    PointRecord2 {
        point_data: PointData,
        colour_data: ColourData,
    },
    PointRecord3 {
        point_data: PointData,
        gps_data: f64,
        colour_data: ColourData,
    },
    PointRecord4 {
        point_data: PointData,
        gps_data: f64,
        wave_packet: WaveformPacket,
    },
    PointRecord5 {
        point_data: PointData,
        gps_data: f64,
        colour_data: ColourData,
        wave_packet: WaveformPacket,
    },
    PointRecord6 {
        point_data: PointData,
        gps_data: f64,
    },
    PointRecord7 {
        point_data: PointData,
        gps_data: f64,
        colour_data: ColourData,
    },
    PointRecord8 {
        point_data: PointData,
        gps_data: f64,
        colour_data: ColourData,
    },
    PointRecord9 {
        point_data: PointData,
        gps_data: f64,
        wave_packet: WaveformPacket,
    },
    PointRecord10 {
        point_data: PointData,
        gps_data: f64,
        colour_data: ColourData,
        wave_packet: WaveformPacket,
    },
}

impl LidarPointRecord {
    pub fn get_point_data(&self) -> PointData {
        return match self {
            LidarPointRecord::PointRecord0 { point_data } => point_data.clone(),
            LidarPointRecord::PointRecord1 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord2 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord3 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord4 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord5 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord6 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord7 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord8 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord9 { point_data, .. } => point_data.clone(),
            LidarPointRecord::PointRecord10 { point_data, .. } => point_data.clone(),
        };
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord0 {
    pub point_data: PointData,
}

impl PointRecord0 {
    pub fn get_format(&self) -> u8 {
        0u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord1 {
    pub point_data: PointData,
    pub gps_data: f64,
}

impl PointRecord1 {
    pub fn get_format(&self) -> u8 {
        1u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord2 {
    pub point_data: PointData,
    pub colour_data: ColourData,
}

impl PointRecord2 {
    pub fn get_format(&self) -> u8 {
        2u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord3 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub colour_data: ColourData,
}

impl PointRecord3 {
    pub fn get_format(&self) -> u8 {
        3u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord4 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub wave_packet: WaveformPacket,
}

impl PointRecord4 {
    pub fn get_format(&self) -> u8 {
        4u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord5 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub colour_data: ColourData,
    pub wave_packet: WaveformPacket,
}

impl PointRecord5 {
    pub fn get_format(&self) -> u8 {
        5u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord6 {
    pub point_data: PointData,
    pub gps_data: f64,
}

impl PointRecord6 {
    pub fn get_format(&self) -> u8 {
        6u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord7 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub colour_data: ColourData,
}

impl PointRecord7 {
    pub fn get_format(&self) -> u8 {
        7u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord8 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub colour_data: ColourData,
}

impl PointRecord8 {
    pub fn get_format(&self) -> u8 {
        8u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord9 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub wave_packet: WaveformPacket,
}

impl PointRecord9 {
    pub fn get_format(&self) -> u8 {
        9u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PointRecord10 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub colour_data: ColourData,
    pub wave_packet: WaveformPacket,
}

impl PointRecord10 {
    pub fn get_format(&self) -> u8 {
        10u8
    }
}

fn fixed_length_string(s: &str, len: usize) -> String {
    let mut ret = "".to_string();
    let mut n = 0;
    for b in s.as_bytes() {
        let mut c = *b as char;
        if *b == 0u8 {
            break;
        }
        if *b > 127u8 {
            // Sorry, but it has to be ASCII data
            c = ' ';
        }
        if n < len {
            ret.push(c);
        } else {
            break;
        }
        n += 1;
    }

    for _ in n..len {
        ret.push('\0');
    }
    ret
}

fn browse_zip_archive<T, F, U>(buf: &mut T, browse_func: F) -> ZipResult<Vec<U>>
where
    T: Read + Seek,
    F: Fn(&ZipFile) -> ZipResult<U>,
{
    let mut archive = ZipArchive::new(buf)?;
    (0..archive.len())
        .map(|i| archive.by_index(i).and_then(|file| browse_func(&file)))
        .collect()
}
