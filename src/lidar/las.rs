/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/01/2017
Last Modified: 07/12/2018
License: MIT
*/

#![allow(dead_code, unused_assignments)]
use super::header::LasHeader;
use super::point_data::{ColourData, PointData, WaveformPacket};
use super::vlr::Vlr;
use crate::raster::geotiff::geokeys::GeoKeys;
use crate::spatial_ref_system::esri_wkt_from_epsg;
use crate::structures::BoundingBox;
use crate::utils::{ByteOrderReader, Endianness};
use byteorder::{LittleEndian, WriteBytesExt};
use chrono::prelude::*;
use core::slice;
use miniz_oxide::deflate::compress_to_vec_zlib;
use miniz_oxide::inflate::decompress_to_vec_zlib;
use std::f64;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufWriter, Cursor, Error, ErrorKind, Seek};
use std::mem;
use std::ops::Index;
use std::path::Path;
use std::str;
use zip::read::{ZipArchive, ZipFile};
use zip::result::ZipResult;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

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
    /// and the `file_mode`, wich can be 'r' (read), 'rh' (read header), and
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
        self.header.number_of_points_by_return_old = [0, 0, 0, 0, 0];

        self.header.x_scale_factor = 0.001;
        self.header.y_scale_factor = 0.001;
        self.header.z_scale_factor = 0.001;

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
            LidarPointRecord::PointRecord0 { point_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
            }
            LidarPointRecord::PointRecord1 {
                point_data,
                gps_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
            }
            LidarPointRecord::PointRecord2 {
                point_data,
                colour_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.colour_data.push(colour_data);
            }
            LidarPointRecord::PointRecord3 {
                point_data,
                gps_data,
                colour_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
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
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
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
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
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
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
            }
            LidarPointRecord::PointRecord7 {
                point_data,
                gps_data,
                colour_data,
            } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
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
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
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
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
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
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
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

    pub fn get_gps_time(&self, index: usize) -> Result<f64, Error> {
        if self.gps_data.len() >= index {
            return Ok(self.gps_data[index]);
        } else {
            return Err(Error::new(ErrorKind::NotFound, "GPS time value not found, possibly because the file point format does not include GPS data."));
        }
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

    pub fn read(&mut self) -> Result<(), Error> {
        if self.file_name.to_lowercase().ends_with(".zlidar") {
            return self.read_zlidar_data();
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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

            let mut next_offset = self.header.offset_to_points as usize;
            let mut point_num = 0;
            let mut block_bytes: u64;
            let mut pt: usize;
            let mut num_points_in_block = 0usize;
            let mut flag = true;
            while flag {
                bor.seek(next_offset);

                let mut field_type = vec![];
                let mut offset = vec![];
                let mut num_bytes = vec![];

                // Start by reading the point data table
                let num_fields = bor.read_u16().expect("Error while reading byte data.");
                block_bytes = 4u64 + 20u64 * num_fields as u64;
                let compression_method = bor.read_u8().expect("Error while reading byte data.");
                if compression_method != 0 {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Unsupported compression method.",
                    ));
                }
                let zldr_version = bor.read_u8().expect("Error while reading byte data.");
                if zldr_version > 1 {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Unsupported ZLidar version {}.", zldr_version),
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
                    block_bytes += 4 - (num_bytes[i] % 4);
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
                            let mut val: f64;
                            let mut vali32: i32;
                            let mut prev_val = 0i32;
                            for j in 0..num_points_in_block {
                                vali32 = bor2.read_i32().expect("Error reading byte data.");
                                val = (vali32 + prev_val) as f64 * self.header.x_scale_factor + self.header.x_offset;

                                // pt = point_num + j;
                                // if pt >= 500_000 && pt < 500_100 {
                                //     println!("{} {} {} {}", (((val - self.header.x_offset) / self.header.x_scale_factor) as i32), vali32, prev_val, val);
                                // }

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
                            let mut val: f64;
                            let mut vali32: i32;
                            let mut prev_val = 0i32;
                            for j in 0..num_points_in_block {
                                vali32 = bor2.read_i32().expect("Error reading byte data.");
                                val = (vali32 + prev_val) as f64 * self.header.y_scale_factor + self.header.y_offset;
                                
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
                            let mut val: f64;
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
                                val = (vali32 + prev_val) as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                                self.gps_data = vec![0f64; self.header.number_of_points as usize];
                            }
                            num_points_in_block = decompressed.len() / 8;
                            let mut bor2 = ByteOrderReader::<Cursor<Vec<u8>>>::new(
                                Cursor::new(decompressed),
                                Endianness::LittleEndian,
                            );
                            let mut val: f64;
                            let mut prev_val = 0f64;
                            for j in 0..num_points_in_block {
                                val = bor2.read_f64().expect("Error reading byte data.") + prev_val;
                                pt = point_num + j;
                                self.gps_data[pt] = val;
                                prev_val = val;
                            }
                        }
                        10 => {
                            // red
                            if self.colour_data.len() == 0 {
                                self.colour_data = vec![Default::default(); self.header.number_of_points as usize];
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
                                self.colour_data = vec![Default::default(); self.header.number_of_points as usize];
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
                                self.colour_data = vec![Default::default(); self.header.number_of_points as usize];
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

            /*
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
                    p.x =
                        bor.read_i32()? as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y =
                        bor.read_i32()? as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z =
                        bor.read_i32()? as f64 * self.header.z_scale_factor + self.header.z_offset;
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
            */
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

        self.header.x_offset = self.header.min_x;
        self.header.y_offset = self.header.min_y;
        self.header.z_offset = self.header.min_z;

        let mut mantissa: usize = (format!("{}", (self.header.max_x - self.header.min_x).floor()))
            .to_string()
            .len();
        let mut dec: f64 = 1.0 / 10_f64.powi(7 - mantissa as i32);
        if self.header.x_scale_factor == 0_f64 {
            self.header.x_scale_factor = dec;
        }

        mantissa = (format!("{}", (self.header.max_y - self.header.min_y).floor()))
            .to_string()
            .len();
        dec = 1.0 / 10_f64.powi(8 - mantissa as i32);
        if self.header.y_scale_factor == 0_f64 {
            self.header.y_scale_factor = dec;
        }

        mantissa = (format!("{}", (self.header.max_z - self.header.min_z).floor()))
            .to_string()
            .len();
        dec = 1.0 / 10_f64.powi(8 - mantissa as i32);
        if self.header.z_scale_factor == 0_f64 {
            self.header.z_scale_factor = dec;
        }

        if !self.file_name.to_lowercase().ends_with(".zip")
            && !self.file_name.to_lowercase().ends_with(".zlidar")
        {
            let f = File::create(&self.file_name)?;
            let mut writer = BufWriter::new(f);

            self.write_data(&mut writer)?;
        } else if self.file_name.to_lowercase().ends_with(".zlidar") {
            let f = File::create(&self.file_name)?;
            let mut writer = BufWriter::new(f);

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
        let mut val: i32;
        match self.header.point_format {
            0 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset)
                        / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;

                    val = ((self.point_data[i].y - self.header.y_offset)
                        / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;

                    val = ((self.point_data[i].z - self.header.z_offset)
                        / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
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
                    val = ((self.point_data[i].x - self.header.x_offset)
                        / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    // y
                    val = ((self.point_data[i].y - self.header.y_offset)
                        / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    // z
                    val = ((self.point_data[i].z - self.header.z_offset)
                        / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
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
                    val = ((self.point_data[i].x - self.header.x_offset)
                        / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;

                    val = ((self.point_data[i].y - self.header.y_offset)
                        / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;

                    val = ((self.point_data[i].z - self.header.z_offset)
                        / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
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
                    val = ((self.point_data[i].x - self.header.x_offset)
                        / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;

                    val = ((self.point_data[i].y - self.header.y_offset)
                        / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;

                    val = ((self.point_data[i].z - self.header.z_offset)
                        / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
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

        let compression_level = 3;
        let mut current_offset = self.header.offset_to_points as u64;
        let mut data_length_in_bytes: u64;
        let mut val: i32;
        // let mut i: usize;
        // let mut ret_num: usize;
        // let mut num_rets: usize;
        // let mut map_index: usize;
        // let mut prev_index: usize;
        let block_size = 50_000usize;
        let mut block_start = 0usize;
        let mut block_end = block_size;
        if block_end > self.header.number_of_points as usize {
            block_end = self.header.number_of_points as usize;
        }
        let mut flag = true;
        // let mut avg_comp = vec![0f64; 10];
        // let mut avg_comp_size = vec![0f64; 10];
        // let mut point_block = 0;
        // let mut total_bytes = 0usize;
        // let mut total_compressed_bytes = 0usize;
        while flag {
            let mut data_code = vec![];
            let mut byte_counts = vec![];
            let mut offsets = vec![];
            let mut output_data = vec![];

            // x
            let mut data = Vec::with_capacity(block_size * 4);
            let mut prev_val = 0i32;
            for i in block_start..block_end {
                val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                data.write_i32::<LittleEndian>(val - prev_val)
                    .expect("Error writing byte data.");
                // if i >= 500_000 && i < 500_100 {
                //     println!("{} {} {} {}", val, val - prev_val, prev_val, self.point_data[i].x);
                // }
                prev_val = val;
            }
            // total_bytes += data.len();
            let mut compressed = compress_to_vec_zlib(&data, compression_level);

            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);

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
            let mut data = Vec::with_capacity(block_size * 4);
            prev_val = 0i32;
            for i in block_start..block_end {
                val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor)
                    as i32;
                data.write_i32::<LittleEndian>(val - prev_val)
                    .expect("Error writing byte data.");
                
                // if i >= 500_000 && i < 500_100 {
                //     println!("{} {} {} {}", val, val - prev_val, prev_val, self.point_data[i].y);
                // }
                prev_val = val;
            }
            // total_bytes += data.len();
            let mut compressed = compress_to_vec_zlib(&data, compression_level);
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
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
            let mut data = Vec::with_capacity(block_size * 4);
            // prev_val = 0i32;
            // let mut prev_late_val = 0i32; // = -1isize;
            // let mut prev_early_val = 0i32; // = -1isize;
            // for i in block_start..block_end {
            //     val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor)
            //         as i32;

            //     prev_val = if self.point_data[i].is_late_return() {
            //         prev_late_val
            //     } else {
            //         prev_early_val
            //     };

            //     data.write_i32::<LittleEndian>(val - prev_val)
            //         .expect("Error writing byte data.");
            //     if self.point_data[i].is_late_return() {
            //         prev_late_val = val;
            //     } else {
            //         prev_early_val = val;
            //     }
            // }

            // prev_val = 0i32;
            // for i in block_start..block_end {
            //     val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor)
            //         as i32;
            //     data.write_i32::<LittleEndian>(val - prev_val)
            //         .expect("Error writing byte data.");

            //     if i >= 500_000 && i < 500_100 {
            //         println!("{} {} {} {}", val, val - prev_val, prev_val, self.point_data[i].z);
            //     }
            //     prev_val = val;
            // }
            
            // prev_val = 0i32;
            // for i in block_start..block_end {
            //     val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor)
            //         as i32;
            //     data.write_i32::<LittleEndian>(val - prev_val)
            //         .expect("Error writing byte data.");

            //     // if i >= 500_000 && i < 500_100 {
            //     //     println!("{} {} {} {}", val, val - prev_val, prev_val, self.point_data[i].z);
            //     // }
            //     if self.point_data[i].is_late_return() {
            //         prev_val = val;
            //     }
            // }

            let mut prev_late_val = 0i32;
            let mut prev_early_val = 0i32;
            for i in block_start..block_end {
                prev_val = if self.point_data[i].is_late_return() {
                    prev_late_val
                } else {
                    prev_early_val
                };
                val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor)
                    as i32;
                data.write_i32::<LittleEndian>(val - prev_val)
                    .expect("Error writing byte data.");
                if self.point_data[i].is_late_return() {
                    prev_late_val = val;
                } else {
                    prev_early_val = val;
                }
            }

            // total_bytes += data.len();
            let mut compressed = compress_to_vec_zlib(&data, compression_level);
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
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
                    data.write_u16::<LittleEndian>(val)
                        .expect("Error writing byte data.");
                }
                // total_bytes += data.len();
                let mut compressed = compress_to_vec_zlib(&data, compression_level);
                data_length_in_bytes = compressed.len() as u64;
                output_data.append(&mut compressed);
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

            // point_bit_field and class_bit_field
            let mut data = Vec::with_capacity(block_size);
            let mut data2 = Vec::with_capacity(block_size);
            for i in block_start..block_end {
                if !self.point_data[i].is_64bit {
                    data.push(self.point_data[i].point_bit_field);
                    data2.push(self.point_data[i].class_bit_field);
                } else {
                    // there is a 64-bit point in the data that we are trying to save as 32-bit.
                    let (point_bit_field, class_bit_field) =
                        self.point_data[i].get_32bit_from_64bit();

                    data.push(point_bit_field);
                    data2.push(class_bit_field);
                }
            }

            // total_bytes += data.len();
            let mut compressed = compress_to_vec_zlib(&data, compression_level);
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
            data_code.push(4u32);
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
            let mut compressed = compress_to_vec_zlib(&data2, compression_level);
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
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
            drop(data2);

            // scan angle
            let mut data = Vec::with_capacity(block_size * 2);
            let mut prev_val = 0i16;
            for i in block_start..block_end {
                data.write_i16::<LittleEndian>(self.point_data[i].scan_angle - prev_val)
                    .expect("Error writing byte data.");
                prev_val = self.point_data[i].scan_angle;
            }
            // total_bytes += data.len() / 2;
            let mut compressed = compress_to_vec_zlib(&data, compression_level);
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
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

            // user data
            if self.use_point_userdata {
                let mut data = Vec::with_capacity(block_size);
                for i in block_start..block_end {
                    data.push(self.point_data[i].user_data);
                }
                // total_bytes += data.len();
                let mut compressed = compress_to_vec_zlib(&data, compression_level);
                data_length_in_bytes = compressed.len() as u64;
                output_data.append(&mut compressed);
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
            for i in block_start..block_end {
                data.write_u16::<LittleEndian>(self.point_data[i].point_source_id)
                    .expect("Error writing byte data.");
            }
            // total_bytes += data.len();
            let mut compressed = compress_to_vec_zlib(&data, compression_level);
            data_length_in_bytes = compressed.len() as u64;
            output_data.append(&mut compressed);
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

            match self.header.point_format {
                0 => {
                    // do nothing; this point type only contains the data parsed above.
                }
                1 => {
                    // gps_data
                    let mut data = Vec::with_capacity(block_size * 8);
                    let mut prev_val = 0f64;
                    for i in block_start..block_end {
                        data.write_f64::<LittleEndian>(self.gps_data[i] - prev_val)
                            .expect("Error writing byte data.");

                        prev_val = self.gps_data[i];
                    }
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data, compression_level);
                    data_length_in_bytes = compressed.len() as u64;
                    output_data.append(&mut compressed);
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
                }
                2 => {
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
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data_r, compression_level);
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
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data_g, compression_level);
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
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data_b, compression_level);
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
                3 => {
                    // gps_data, colour_data
                    let mut data = Vec::with_capacity(block_size * 8);
                    let mut data_r = Vec::with_capacity(block_size * 2);
                    let mut data_g = Vec::with_capacity(block_size * 2);
                    let mut data_b = Vec::with_capacity(block_size * 2);
                    for i in block_start..block_end {
                        data.write_f64::<LittleEndian>(self.gps_data[i])
                            .expect("Error writing byte data.");
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
                    // gps_data
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data, compression_level);
                    data_length_in_bytes = compressed.len() as u64;
                    output_data.append(&mut compressed);
                    data_code.push(9u32);
                    byte_counts.push(data_length_in_bytes);
                    offsets.push(current_offset);
                    current_offset += data_length_in_bytes;
                    if data_length_in_bytes % 4 > 0 {
                        for _ in 0..(4 - (data_length_in_bytes % 4)) {
                            output_data.push(0u8);
                            current_offset += 1;
                        }
                    }

                    // r
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data_r, compression_level);
                    data_length_in_bytes = compressed.len() as u64;
                    output_data.append(&mut compressed);
                    data_code.push(10u32);
                    byte_counts.push(data_length_in_bytes);
                    offsets.push(current_offset);
                    current_offset += data_length_in_bytes;
                    if data_length_in_bytes % 4 > 0 {
                        for _ in 0..(4 - (data_length_in_bytes % 4)) {
                            output_data.push(0u8);
                            current_offset += 1;
                        }
                    }

                    // g
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data_g, compression_level);
                    data_length_in_bytes = compressed.len() as u64;
                    output_data.append(&mut compressed);
                    data_code.push(11u32);
                    byte_counts.push(data_length_in_bytes);
                    offsets.push(current_offset);
                    current_offset += data_length_in_bytes;
                    if data_length_in_bytes % 4 > 0 {
                        for _ in 0..(4 - (data_length_in_bytes % 4)) {
                            output_data.push(0u8);
                            current_offset += 1;
                        }
                    }

                    // b
                    // total_bytes += data.len();
                    let mut compressed = compress_to_vec_zlib(&data_b, compression_level);
                    data_length_in_bytes = compressed.len() as u64;
                    output_data.append(&mut compressed);
                    data_code.push(12u32);
                    byte_counts.push(data_length_in_bytes);
                    offsets.push(current_offset);
                    current_offset += data_length_in_bytes;
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
                _ => {
                    return Err(Error::new(ErrorKind::Other, "Unsupported point format"));
                }
            }

            // Now write the data. Start with the point data table
            let table_length = 4u64 + data_code.len() as u64 * 20u64;
            writer
                .write_u16::<LittleEndian>(data_code.len() as u16)
                .expect("Error writing byte data to file.");
            let compression_method = 0u8; // DEFLATE (ZLIB)
            writer
                .write_u8(compression_method)
                .expect("Error writing byte data to file.");
            writer.write_u8(1u8).expect("Error writing byte data to file."); // zlidar version number

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

                /*
                let byte_size = if data_code[i] < 3 {
                    4usize
                } else if data_code[i] == 3 {
                    2usize
                } else if data_code[i] < 8 {
                    1usize
                } else if data_code[i] == 8 {
                    2usize
                } else {
                    8usize
                };
                avg_comp[data_code[i] as usize] +=
                    100f64 * (byte_counts[i] as f64 / (block_size * byte_size) as f64);
                avg_comp_size[data_code[i] as usize] += byte_counts[i] as f64
                */
            }
            // total_compressed_bytes += output_data.len();
            writer
                .write_all(&output_data)
                .expect("Error writing byte data to file.");

            current_offset += table_length;

            // point_block += 1;
            block_start = block_end;
            if block_start >= self.header.number_of_points as usize {
                flag = false;
            }
            block_end += block_size;
            if block_end > self.header.number_of_points as usize {
                block_end = self.header.number_of_points as usize;
            }
        }

        /*
        println!("Average compressed size:");
        for a in 0..10usize {
            avg_comp[a] /=  point_block as f64;
            avg_comp_size[a] /= point_block as f64;
            match a {
                0usize => println!("x: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                1usize => println!("y: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                2usize => println!("z: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                3usize => println!("intensity: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                4usize => println!("point byte: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                5usize => println!("classification: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                6usize => println!("scan angle: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                7usize => println!("user data: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                8usize => println!("source id: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
                _ => println!("GPS time: {:.1} ({:.2})", avg_comp_size[a], avg_comp[a]),
            }
        }
        println!("Overall: {} ({:.2})", total_compressed_bytes, 100.0 * total_compressed_bytes as f64 / total_bytes as f64);
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

    // pub fn get_geokeys(self) -> String {
    //     return self.geokeys.to_string();
    // }
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

    /// Returns the co-ordinate reference system method used within the file.
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
