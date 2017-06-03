#![allow(dead_code, unused_assignments)]
extern crate time;

use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::fmt;
use std::f64;
use std::io::BufWriter;
use std::fs::File;
use std::fs;
use std::mem;
use std::str;
use lidar::header::LasHeader;
use lidar::point_data::{ ClassificationBitField, PointBitField, PointData, RgbData, WaveformPacket };
use lidar::vlr::Vlr;
use raster::geotiff::geokeys::GeoKeys;
use std::ops::Index;

#[derive(Default, Clone)]
pub struct LasFile {
    file_name: String,
    file_mode: String,
    pub header: LasHeader,
    pub vlr_data: Vec<Vlr>,
    point_data: Vec<PointData>,
    // point_buffer_size: usize,
    gps_data: Vec<f64>,
    rgb_data: Vec<RgbData>,
    waveform_data: Vec<WaveformPacket>,
    pub geokeys: GeoKeys,
    // starting_point: usize,
    header_is_set: bool,
    pub use_point_intensity: bool,
    pub use_point_userdata: bool,
}

impl Index<usize> for LasFile {
    type Output = PointData;

    fn index<'a>(&'a self, _index: usize) -> &'a PointData {
        &self.point_data[_index]
    }
}

impl LasFile {

    pub fn new<'a>(file_name: &'a str, file_mode: &'a str) -> Result<LasFile, Error> { //LasFile {
        let mut lf = LasFile { file_name: file_name.to_string(), ..Default::default() };
        lf.file_mode = file_mode.to_lowercase();
        if lf.file_mode == "r" || lf.file_mode == "rh"  {
            try!(lf.read());
        } else {
            lf.file_mode = "w".to_string();
        }
        // lf.point_buffer_size = 1000000;
        lf.use_point_intensity = true;
        lf.use_point_userdata = true;
        Ok(lf)
    }

    /// This function returns a new LasFile that has been initialized using another
    /// LasFile.
    /// Input Parameters:
    /// * file_name: The name of the LAS file to be created.
    /// * input: An existing LAS file.
    ///
    /// Output:
    /// * A LasFile struct, initialized with the header and VLRs of the input file.
    pub fn initialize_using_file<'a>(file_name: &'a str, input: &'a LasFile) -> LasFile {
        let mut output = LasFile { file_name: file_name.to_string(), ..Default::default() };
        output.file_mode = "w".to_string();
        output.use_point_intensity = true;
        output.use_point_userdata = true;

        output.add_header(input.header.clone());

        // Copy the VLRs
        for i in 0..(input.header.number_of_vlrs as usize) {
            output.add_vlr(input.vlr_data[i].clone());
        }

        output
    }

    pub fn add_header(&mut self, header: LasHeader) {
        if self.file_mode == "r" { return; }
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

		self.header.system_id = "LiDAR Tools by John Lindsay     ".to_string();
		self.header.generating_software = "LiDAR Tools                     ".to_string();
		self.header.number_of_points_by_return = [0, 0, 0, 0, 0];

		self.header.x_scale_factor = 0.0001;
		self.header.y_scale_factor = 0.0001;
		self.header.z_scale_factor = 0.0001;

        self.header_is_set = true;
    }

    pub fn add_vlr(&mut self, vlr: Vlr) {
        if self.file_mode == "r" { return; }
        // the header must be set before you can add VLRs
        if !self.header_is_set {
            panic!("The header of a LAS file must be added before any VLRs. Please see add_header().");
        }
        self.vlr_data.push(vlr);
        self.header.number_of_vlrs += 1;
    }

    pub fn add_point_record(&mut self, point: LidarPointRecord) {
        if self.file_mode == "r" { return; }
        if !self.header_is_set {
            panic!("The header of a LAS file must be added before any point records. Please see add_header().");
        }
        let mut which_return = 0_usize;
        let x: f64;
        let y: f64;
        let z: f64;
        match point {
            LidarPointRecord::PointRecord0 { point_data }  => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.bit_field.return_number() as usize;
            },
            LidarPointRecord::PointRecord1 { point_data, gps_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.bit_field.return_number() as usize;
                self.gps_data.push(gps_data);
            },
            LidarPointRecord::PointRecord2 { point_data, rgb_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.bit_field.return_number() as usize;
                self.rgb_data.push(rgb_data);
            },
            LidarPointRecord::PointRecord3 { point_data, gps_data, rgb_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.bit_field.return_number() as usize;
                self.gps_data.push(gps_data);
                self.rgb_data.push(rgb_data);
            },
        }

        if x < self.header.min_x { self.header.min_x = x; }
        if x > self.header.max_x { self.header.max_x = x; }
        if y < self.header.min_y { self.header.min_y = y; }
        if y > self.header.max_y { self.header.max_y = y; }
        if z < self.header.min_z { self.header.min_z = z; }
        if z > self.header.max_z { self.header.max_z = z; }

        self.header.number_of_points += 1;
        if which_return == 0 { which_return = 1; }
        self.header.number_of_points_by_return[which_return-1] += 1;
    }

    pub fn get_record(&self, index: usize) -> LidarPointRecord {
        let lpr: LidarPointRecord;
        match self.header.point_format {
            0 => {
                lpr = LidarPointRecord::PointRecord0 { point_data: self.point_data[index] };
            },
            1 => {
                lpr = LidarPointRecord::PointRecord1 { point_data: self.point_data[index],
                    gps_data: self.gps_data[index] };
            },
            2 => {
                lpr = LidarPointRecord::PointRecord2 { point_data: self.point_data[index],
                    rgb_data: self.rgb_data[index] };
            },
            3 => {
                lpr = LidarPointRecord::PointRecord3 { point_data: self.point_data[index],
                    gps_data: self.gps_data[index], rgb_data: self.rgb_data[index] };
            },
            _ => {
                panic!("Unsupported point format");
            },
        }
        lpr
    }

    pub fn get_point_info(&self, index: usize) -> PointData {
        self.point_data[index]
    }

    pub fn get_rgb(&self, index: usize) -> Result<RgbData, Error> {
        if self.rgb_data.len() >= index {
            return Ok(self.rgb_data[index]);
        } else {
            return Err(Error::new(ErrorKind::NotFound, "RGB value not found, possibly because the file point format does not include colour data."));
        }
    }

    pub fn get_gps_time(&self, index: usize) -> Result<f64, Error> {
        if self.gps_data.len() >= index {
            return Ok(self.gps_data[index]);
        } else {
            return Err(Error::new(ErrorKind::NotFound, "GPS time value not found, possibly because the file point format does not include GPS data."));
        }
    }

    pub fn read(&mut self) -> Result<(), Error> {

        // // See if the file exists. If not, raise error.
        // fs::metadata(&self.file_name)

        let mut f = File::open(&self.file_name)?;
        let metadata = fs::metadata(&self.file_name)?;
        let file_size: usize = metadata.len() as usize;
        let mut buffer = vec![0; file_size];

        // read the file's bytes into a buffer
        f.read(&mut buffer)?;

        self.header.project_id_used = true;
        self.header.version_major = buffer[24];
        self.header.version_minor = buffer[25];
        if self.header.version_major < 1 || self.header.version_major > 2 || self.header.version_minor > 5 {
            // There's something wrong. It could be that the project ID values are not included in the header.
            self.header.version_major = buffer[8];
            self.header.version_minor = buffer[9];
            if self.header.version_major < 1 || self.header.version_major > 2 || self.header.version_minor > 5 {
                // There's something very wrong. Throw an error.
                return Err(Error::new(ErrorKind::Other, "Either the file is formatted incorrectly or it is an unsupported LAS version."));
            }
            self.header.project_id_used = false;
        }
        unsafe {

            //////////////////////////
            // Read the File Header //
            //////////////////////////
            let mut offset: usize = 0;
            self.header.file_signature = String::from_utf8_lossy(&buffer[offset..offset+4]).to_string();
            if self.header.file_signature != "LASF" {
                return Err(Error::new(ErrorKind::Other, "Either the file is formatted incorrectly or it is an unsupported LAS version."));
            }
            offset += 4;
            self.header.file_source_id = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
            offset += 2;
            let ge_val = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
            self.header.global_encoding = GlobalEncodingField { value: ge_val};
            offset += 2;
            if self.header.project_id_used {
                self.header.project_id1 = mem::transmute::<[u8; 4], u32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]);
                offset += 4;
                self.header.project_id2 = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                offset += 2;
                self.header.project_id3 = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                offset += 2;
                for i in 0..8 {
                    self.header.project_id4[i] = buffer[offset + i];
                }
                offset += 8;
            }
            // The version major and minor are read earlier.
            // Two bytes that must be added to the offset here.
            offset += 2;
            //self.header.project_id4 = String::from_utf8_lossy(&buffer[16..24]).trim().to_string();
            self.header.system_id = String::from_utf8_lossy(&buffer[offset..offset+32]).trim().to_string();
            offset += 32;
            self.header.generating_software = String::from_utf8_lossy(&buffer[offset..offset+32]).trim().to_string();
            offset += 32;
            // self.header.system_id = String::from_utf8_lossy(&buffer[26..58]).trim().to_string();
            // self.header.generating_software = String::from_utf8_lossy(&buffer[58..90]).trim().to_string();
            self.header.file_creation_day = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
            offset += 2;
            self.header.file_creation_year = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
            offset += 2;
            self.header.header_size = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
            offset += 2;
            self.header.offset_to_points = mem::transmute::<[u8; 4], u32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]);
            offset += 4;
            self.header.number_of_vlrs = mem::transmute::<[u8; 4], u32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]);
            offset += 4;
            self.header.point_format = buffer[offset];
            offset += 1;
            self.header.point_record_length = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
            offset += 2;
            self.header.number_of_points = mem::transmute::<[u8; 4], u32>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]);
            offset += 4;

            // let mut num_returns = 5;
            // if self.header.version_major == 1_u8 && self.header.version_minor > 3_u8 {
            //     num_returns = 7;
            // }
            // offset = 111;
            for i in 0..5 {
                self.header.number_of_points_by_return[i] = mem::transmute::<[u8; 4], u32>([buffer[offset + i * 4], buffer[offset + i * 4 + 1], buffer[offset + i * 4 + 2], buffer[offset + i * 4 + 3]]);
                // self.header.number_of_points_by_return.push(mem::transmute::<[u8; 4], u32>([buffer[offset + i * 4], buffer[offset + i * 4 + 1], buffer[offset + i * 4 + 2], buffer[offset + i * 4 + 3]]));
            }
            offset += 5 * 4;
            self.header.x_scale_factor = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.y_scale_factor = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.z_scale_factor = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.x_offset = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.y_offset = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.z_offset = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.max_x = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.min_x = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.max_y = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.min_y = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.max_z = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            offset += 8;
            self.header.min_z = mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);

            if self.header.version_major == 1 && self.header.version_minor == 3 {
                offset += 8;
                self.header.waveform_data_start = mem::transmute::<[u8; 8], u64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
            }

            ///////////////////////
            // Read the VLR data //
            ///////////////////////
            offset = self.header.header_size as usize;
            //self.vlr_data = vec![Vlr{0'u16, "".to_string(), 0'u16, 0'u16, "".to_string()}; self.header.number_of_vlrs as usize];
            for _ in 0..self.header.number_of_vlrs {
                let mut vlr: Vlr = Default::default();
                vlr.reserved = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                offset += 2;
                vlr.user_id = String::from_utf8_lossy(&buffer[offset..offset+16]).trim().to_string();
                offset += 16;
                vlr.record_id = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                offset += 2;
                vlr.record_length_after_header = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                offset += 2;
                vlr.description = String::from_utf8_lossy(&buffer[offset..offset+32]).trim().to_string();
                offset += 32;
                // get the byte data
                for i in 0..vlr.record_length_after_header {
                    vlr.binary_data.push(buffer[offset + i as usize]);
                }
                offset += vlr.record_length_after_header as usize;

                if vlr.record_id == 34_735 {
                    self.geokeys.add_key_directory(&vlr.binary_data);
                } else if vlr.record_id == 34_736 {
                    self.geokeys.add_double_params(&vlr.binary_data);
                } else if vlr.record_id == 34_737 {
                    self.geokeys.add_ascii_params(&vlr.binary_data);
                }
                self.vlr_data.push(vlr);
            }

            if self.file_mode != "rh" { // file_mode = "rh" does not read points, only the header.
                /////////////////////////
                // Read the point data //
                /////////////////////////
                // self.starting_point = 0;
                // let _ = self.read_points().unwrap();

                // Intensity and userdata are both optional. Figure out if they need to be read.
                // The only way to do this is to compare the point record length by point format
                let rec_lengths = [ [20_u16, 18_u16, 19_u16, 17_u16],
                                    [28_u16, 26_u16, 27_u16, 25_u16],
                                    [26_u16, 24_u16, 25_u16, 23_u16],
                                    [34_u16, 32_u16, 33_u16, 31_u16] ];

                if self.header.point_record_length == rec_lengths[self.header.point_format as usize][0] {
                    self.use_point_intensity = true;
                    self.use_point_userdata = true;
                } else if self.header.point_record_length == rec_lengths[self.header.point_format as usize][1] {
                    self.use_point_intensity = false;
                    self.use_point_userdata = true;
                } else if self.header.point_record_length == rec_lengths[self.header.point_format as usize][2] {
                    self.use_point_intensity = true;
                    self.use_point_userdata = false;
                } else if self.header.point_record_length == rec_lengths[self.header.point_format as usize][3] {
                    self.use_point_intensity = false;
                    self.use_point_userdata = false;
                }


                for i in 0..self.header.number_of_points {
                    offset = (self.header.offset_to_points + (i as u32) * (self.header.point_record_length as u32)) as usize;
                    let mut p: PointData = Default::default();
                    p.x = ((mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2],
                        buffer[offset+3]])) as f64) * self.header.x_scale_factor + self.header.x_offset;
                    offset += 4;
                    p.y = ((mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2],
                        buffer[offset+3]])) as f64) * self.header.y_scale_factor + self.header.y_offset;
                    offset += 4;
                    p.z = ((mem::transmute::<[u8; 4], i32>([buffer[offset], buffer[offset+1], buffer[offset+2],
                        buffer[offset+3]])) as f64) * self.header.z_scale_factor + self.header.z_offset;
                    offset += 4;
                    if self.use_point_intensity {
                        p.intensity = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        offset += 2;
                    }
                    p.bit_field =  PointBitField { value: buffer[offset] };
                    offset += 1;
                    p.class_bit_field = ClassificationBitField { value: buffer[offset] };
                    offset += 1;
                    p.scan_angle = mem::transmute::<[u8; 1], i8>([buffer[offset]]);
                    offset += 1;
                    if self.use_point_userdata {
                        p.user_data = buffer[offset];
                        offset += 1;
                    }
                    p.point_source_id = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                    self.point_data.push(p);
                }

                if self.header.point_format == 1 {
                    // read the GPS data
                    for i in 0..self.header.number_of_points {
                        offset = (self.header.offset_to_points + 20u32 + (i as u32) * (self.header.point_record_length as u32)) as usize;
                        self.gps_data.push(mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]));
                    }
                } else if self.header.point_format == 2 {
                    // read the RGB data
                    for i in 0..self.header.number_of_points {
                        offset = (self.header.offset_to_points + 20u32 + (i as u32) * (self.header.point_record_length as u32)) as usize;
                        let mut rgb: RgbData = Default::default();
                        rgb.red = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        offset += 2;
                        rgb.green = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        offset += 2;
                        rgb.blue = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        self.rgb_data.push(rgb);
                        // let r: u16 = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        // offset += 2;
                        // let g: u16 = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        // offset += 2;
                        // let b: u16 = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        // self.rgb_data.push(RgbData { red: r, green: g, blue: b });
                    }
                } else if self.header.point_format == 3 {
                    for i in 0..self.header.number_of_points {
                        // read the GPS data
                        offset = (self.header.offset_to_points + 20u32 + (i as u32) * (self.header.point_record_length as u32)) as usize;
                        self.gps_data.push(mem::transmute::<[u8; 8], f64>([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3], buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]));
                        offset += 8;
                        // read the RGB data
                        let mut rgb: RgbData = Default::default();
                        rgb.red = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        offset += 2;
                        rgb.green = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        offset += 2;
                        rgb.blue = mem::transmute::<[u8; 2], u16>([buffer[offset], buffer[offset+1]]);
                        self.rgb_data.push(rgb);
                    }
                } else if self.header.point_format == 4 {

                }
            }

        }

        Ok(())
    }

    pub fn write(&mut self) -> Result<(), Error> {
        if self.file_mode == "r" {
            return Err(Error::new(ErrorKind::Other, "The file was opened in read-only mode"));
        }
        if !self.header_is_set {
            return Err(Error::new(ErrorKind::Other, "The header of a LAS file must be added before any point records. Please see add_header()."));
        }

        self.header.x_offset = self.header.min_x;
        self.header.y_offset = self.header.min_y;
        self.header.z_offset = self.header.min_z;

        let mut mantissa: usize = (format!("{}", (self.header.max_x - self.header.min_x).floor())).to_string().len();
        let mut dec: f64 = 1.0 / 10_f64.powi(8 - mantissa as i32);
        if self.header.x_scale_factor == 0_f64 { self.header.x_scale_factor = dec; }

        mantissa = (format!("{}", (self.header.max_y - self.header.min_y).floor())).to_string().len();
        dec = 1.0 / 10_f64.powi(8 - mantissa as i32);
        if self.header.y_scale_factor == 0_f64 { self.header.y_scale_factor = dec; }

        mantissa = (format!("{}", (self.header.max_z - self.header.min_z).floor())).to_string().len();
        dec = 1.0 / 10_f64.powi(8 - mantissa as i32);
        if self.header.z_scale_factor == 0_f64 { self.header.z_scale_factor = dec; }

        let f = File::create(&self.file_name)?;
        let mut writer = BufWriter::new(f);

        /////////////////////////////////
        // Write the header to the file /
        /////////////////////////////////
        let mut u16_bytes: [u8; 2];
        let mut u32_bytes: [u8; 4];
        let mut u64_bytes: [u8; 8];

        self.header.file_signature = "LASF".to_string();
        writer.write(self.header.file_signature.as_bytes())?;

        u16_bytes = unsafe {mem::transmute(self.header.file_source_id)};
        writer.write(&u16_bytes)?;

        u16_bytes = unsafe { mem::transmute(self.header.global_encoding) };
        writer.write(&u16_bytes)?;

        if self.header.project_id_used {
            u32_bytes = unsafe { mem::transmute(self.header.project_id1) };
            writer.write(&u32_bytes)?;

            u16_bytes = unsafe { mem::transmute(self.header.project_id2) };
            writer.write(&u16_bytes)?;

            u16_bytes = unsafe { mem::transmute(self.header.project_id3) };
            writer.write(&u16_bytes)?;

            u64_bytes = unsafe { mem::transmute(self.header.project_id4) };
            writer.write(&u64_bytes)?;
        }

        self.header.version_major = 1u8;
        let mut u8_bytes: [u8; 1] = unsafe {mem::transmute(self.header.version_major)};
        writer.write(&u8_bytes)?;

        self.header.version_minor = 3u8;
        u8_bytes = unsafe {mem::transmute(self.header.version_minor)};
        writer.write(&u8_bytes)?;

        if self.header.system_id.len() == 0 {
            self.header.system_id = fixed_length_string("OTHER", 32);
        } else if !self.header.system_id.len() != 32 {
            self.header.system_id = fixed_length_string(&(self.header.system_id), 32);
        }
        writer.write(self.header.system_id.as_bytes())?; //string_bytes));

        self.header.generating_software = fixed_length_string("whitebox_tools by John Lindsay", 32);
        //string_bytes = unsafe { mem::transmute("libgeospatial by John Lindsay   ") };
        writer.write(self.header.generating_software.as_bytes())?;

        let now = time::now();
        self.header.file_creation_day = now.tm_yday as u16;
        u16_bytes = unsafe { mem::transmute(self.header.file_creation_day) };
        writer.write(&u16_bytes)?;

        self.header.file_creation_year = (now.tm_year + 1900) as u16;
        u16_bytes = unsafe { mem::transmute(self.header.file_creation_year) };
        writer.write(&u16_bytes)?;

        self.header.header_size = 235;
        u16_bytes = unsafe { mem::transmute(self.header.header_size) };
        writer.write(&u16_bytes)?;

        // figure out the offset to points
        let mut total_vlr_size = 54 * self.header.number_of_vlrs;
        for i in 0..(self.header.number_of_vlrs as usize) {
            total_vlr_size += self.vlr_data[i].record_length_after_header as u32;
        }
        self.header.offset_to_points = 235 + total_vlr_size;
        u32_bytes = unsafe { mem::transmute(self.header.offset_to_points) };
        writer.write(&u32_bytes)?;

        u32_bytes = unsafe { mem::transmute(self.header.number_of_vlrs) };
        writer.write(&u32_bytes)?;

        u8_bytes = unsafe {mem::transmute(self.header.point_format)};
        writer.write(&u8_bytes)?;

        // Intensity and userdata are both optional. Figure out if they need to be read.
        // The only way to do this is to compare the point record length by point format
        let rec_lengths = [ [20_u16, 18_u16, 19_u16, 17_u16],
                            [28_u16, 26_u16, 27_u16, 25_u16],
                            [26_u16, 24_u16, 25_u16, 23_u16],
                            [34_u16, 32_u16, 33_u16, 31_u16] ];

        if self.use_point_intensity && self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][0];
        } else if !self.use_point_intensity && self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][1];
        } else if self.use_point_intensity && !self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][2];
        } else { //if !self.use_point_intensity && !self.use_point_userdata {
            self.header.point_record_length = rec_lengths[self.header.point_format as usize][3];
        }

        u16_bytes = unsafe { mem::transmute(self.header.point_record_length) };
        writer.write(&u16_bytes)?;

        u32_bytes = unsafe { mem::transmute(self.header.number_of_points) };
        writer.write(&u32_bytes)?;

        for i in 0..5 {
            u32_bytes = unsafe { mem::transmute(self.header.number_of_points_by_return[i]) };
            writer.write(&u32_bytes)?;
        }

        u64_bytes = unsafe { mem::transmute(self.header.x_scale_factor) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.y_scale_factor) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.z_scale_factor) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.x_offset) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.y_offset) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.z_offset) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_x) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_x) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_y) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_y) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.max_z) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.min_z) };
        writer.write(&u64_bytes)?;

        u64_bytes = unsafe { mem::transmute(self.header.waveform_data_start) };
        writer.write(&u64_bytes)?;

        ///////////////////////////////
        // Write the VLRs to the file /
        ///////////////////////////////
        for i in 0..(self.header.number_of_vlrs as usize) {
            let vlr = self.vlr_data[i].clone();
            u16_bytes = unsafe { mem::transmute(vlr.reserved) };
            writer.write(&u16_bytes)?;

            let user_id: &str = &vlr.user_id;
            //string_bytes = unsafe { mem::transmute(user_id) };
            writer.write(user_id.as_bytes())?; //string_bytes));

            u16_bytes = unsafe { mem::transmute(vlr.record_id) };
            writer.write(&u16_bytes)?;

            u16_bytes = unsafe { mem::transmute(vlr.record_length_after_header) };
            writer.write(&u16_bytes)?;

            let description: &str = &vlr.description;
            //string_bytes = unsafe { mem::transmute(description) };
            writer.write(description.as_bytes())?;

            writer.write(&vlr.binary_data)?;
        }

        ////////////////////////////////
        // Write the point to the file /
        ////////////////////////////////
        let mut val: i32;
        match self.header.point_format {
            0 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle)};
                    writer.write(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
                        writer.write(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write(&u16_bytes)?;
                }
            },
            1 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle)};
                    writer.write(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
                        writer.write(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write(&u16_bytes)?;

                    u64_bytes = unsafe { mem::transmute(self.gps_data[i]) };
                    writer.write(&u64_bytes)?;
                }
            },
            2 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle)};
                    writer.write(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
                        writer.write(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.rgb_data[i].red) };
                    writer.write(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.rgb_data[i].green) };
                    writer.write(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.rgb_data[i].blue) };
                    writer.write(&u16_bytes)?;
                }
            },
            3 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write(&u32_bytes)?;

                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field.value)};
                    writer.write(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle)};
                    writer.write(&u8_bytes)?;

                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
                        writer.write(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write(&u16_bytes)?;

                    u64_bytes = unsafe { mem::transmute(self.gps_data[i]) };
                    writer.write(&u64_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.rgb_data[i].red) };
                    writer.write(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.rgb_data[i].green) };
                    writer.write(&u16_bytes)?;

                    u16_bytes = unsafe { mem::transmute(self.rgb_data[i].blue) };
                    writer.write(&u16_bytes)?;
                }
            },
            _ => {
                return Err(Error::new(ErrorKind::Other, "Unsupported point format"));
            },
        }

        Ok(())
    }

    pub fn get_vlr_data_as_string(&self) -> String {
        let mut s = "".to_string();
        let mut i : usize = 1;
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
        write!(f, "{}", format!("File Name: {}\n{}", self.file_name, &self.header))
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
        write!(f,
"    GPS time={:?},
    Waveform data internal={},
    Waveform data external={},
    Return data synthetic={},
    CRS method={:?}", self.gps_time(), self.waveform_data_internal(), self.waveform_data_external(),
    self.return_data_synthetic(), self.coordinate_reference_system_method())
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
    PointRecord0 { point_data: PointData },
    PointRecord1 { point_data: PointData, gps_data: f64 },
    PointRecord2 { point_data: PointData, rgb_data: RgbData },
    PointRecord3 { point_data: PointData, gps_data: f64, rgb_data: RgbData }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct  PointRecord0 {
    pub point_data: PointData,
}

impl PointRecord0 {
    pub fn get_format(&self) -> u8 {
        0u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct  PointRecord1 {
    pub point_data: PointData,
    pub gps_data: f64,
}

impl PointRecord1 {
    pub fn get_format(&self) -> u8 {
        1u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct  PointRecord2 {
    pub point_data: PointData,
    pub rgb_data: RgbData,
}

impl PointRecord2 {
    pub fn get_format(&self) -> u8 {
        2u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct  PointRecord3 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub rgb_data: RgbData,
}

impl PointRecord3 {
    pub fn get_format(&self) -> u8 {
        3u8
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct  PointRecord4 {
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
pub struct  PointRecord5 {
    pub point_data: PointData,
    pub gps_data: f64,
    pub rgb_data: RgbData,
    pub wave_packet: WaveformPacket,
}

impl PointRecord5 {
    pub fn get_format(&self) -> u8 {
        5u8
    }
}

fn fixed_length_string(s: &str, len: usize) -> String {
    //let array: &[u8: 32];
    let l = s.len();
    let mut ret: String = "".to_owned();
    if l < len {
        // add spaces to end
        ret = s.to_string();
        for _ in 0..len-l {
            ret.push_str("\0");
        }
    } else {
        // truncate string
        ret = s[0..len].to_string(); // could use 'truncate' method as well.
    }
    ret
}
