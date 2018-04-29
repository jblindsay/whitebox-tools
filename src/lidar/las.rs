#![allow(dead_code, unused_assignments)]
extern crate time;
extern crate zip;

use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::fmt;
use std::f64;
use std::io::BufWriter;
use std::fs::File;
use std::fs;
use std::mem;
use std::path::Path;
use std::str;
use lidar::header::LasHeader;
use lidar::point_data::{ PointData, ColourData, WaveformPacket };
use lidar::vlr::Vlr;
use raster::geotiff::geokeys::GeoKeys;
use structures::BoundingBox;
use io_utils::{ByteOrderReader, Endianness};
use std::ops::Index;
use std::io::Seek;
use self::zip::result::ZipResult;
use self::zip::CompressionMethod;
use self::zip::read::{ ZipArchive, ZipFile };
use self::zip::write::{FileOptions, ZipWriter};


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

		self.header.system_id = "WhiteboxTools by John Lindsay   ".to_string();
		self.header.generating_software = "WhiteboxTools                   ".to_string();
		self.header.number_of_points_by_return_old = [0, 0, 0, 0, 0];

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
                which_return = point_data.return_number() as usize;
            },
            LidarPointRecord::PointRecord1 { point_data, gps_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
            },
            LidarPointRecord::PointRecord2 { point_data, colour_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.colour_data.push(colour_data);
            },
            LidarPointRecord::PointRecord3 { point_data, gps_data, colour_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
            },
            LidarPointRecord::PointRecord4 { point_data, gps_data, wave_packet } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.waveform_data.push(wave_packet);
            },
            LidarPointRecord::PointRecord5 { point_data, gps_data, colour_data, wave_packet } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
                self.waveform_data.push(wave_packet);
            },
            LidarPointRecord::PointRecord6 { point_data, gps_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
            },
            LidarPointRecord::PointRecord7 { point_data, gps_data, colour_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
            },
            LidarPointRecord::PointRecord8 { point_data, gps_data, colour_data } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
            },
            LidarPointRecord::PointRecord9 { point_data, gps_data, wave_packet } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.waveform_data.push(wave_packet);
            },
            LidarPointRecord::PointRecord10 { point_data, gps_data, colour_data, wave_packet } => {
                self.point_data.push(point_data);
                x = point_data.x;
                y = point_data.y;
                z = point_data.z;
                which_return = point_data.return_number() as usize;
                self.gps_data.push(gps_data);
                self.colour_data.push(colour_data);
                self.waveform_data.push(wave_packet);
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
        if which_return <= 5 {
            self.header.number_of_points_by_return[which_return-1] += 1;
        }
    }

    pub fn get_record(&self, index: usize) -> LidarPointRecord {
        let lpr: LidarPointRecord;
        match self.header.point_format {
            0 => {
                lpr = LidarPointRecord::PointRecord0 { point_data: self.point_data[index] };
            },
            1 => {
                lpr = LidarPointRecord::PointRecord1 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index] 
                };
            },
            2 => {
                lpr = LidarPointRecord::PointRecord2 { 
                    point_data: self.point_data[index],
                    colour_data: self.colour_data[index] 
                };
            },
            3 => {
                lpr = LidarPointRecord::PointRecord3 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index], 
                    colour_data: self.colour_data[index] 
                };
            },
            4 => {
                lpr = LidarPointRecord::PointRecord4 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index], 
                    wave_packet: self.waveform_data[index] 
                };
            },
            5 => {
                lpr = LidarPointRecord::PointRecord5 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index], 
                    colour_data: self.colour_data[index],
                    wave_packet: self.waveform_data[index] 
                };
            },
            6 => {
                lpr = LidarPointRecord::PointRecord6 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index]
                };
            },
            7 => {
                lpr = LidarPointRecord::PointRecord7 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index], 
                    colour_data: self.colour_data[index]
                };
            },
            8 => {
                lpr = LidarPointRecord::PointRecord8 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index], 
                    colour_data: self.colour_data[index]
                };
            },
            9 => {
                lpr = LidarPointRecord::PointRecord9 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index], 
                    wave_packet: self.waveform_data[index] 
                };
            },
            10 => {
                lpr = LidarPointRecord::PointRecord10 { 
                    point_data: self.point_data[index],
                    gps_data: self.gps_data[index], 
                    colour_data: self.colour_data[index],
                    wave_packet: self.waveform_data[index] 
                };
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

    pub fn get_rgb(&self, index: usize) -> Result<ColourData, Error> {
        if self.colour_data.len() >= index {
            return Ok(self.colour_data[index]);
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

    pub fn get_extent(&self) -> BoundingBox {
        BoundingBox {
            min_x: self.header.min_x,
            max_x: self.header.max_x,
            min_y: self.header.min_y,
            max_y: self.header.max_y
        }
    }

    pub fn read(&mut self) -> Result<(), Error> {
        let buffer = match self.file_name.to_lowercase().ends_with(".zip") {
            false => {
                let mut f = File::open(&self.file_name)?;
                let metadata = fs::metadata(&self.file_name)?;
                let file_size: usize = metadata.len() as usize;
                let mut buffer = vec![0; file_size];

                // read the file's bytes into a buffer
                f.read(&mut buffer)?;
                buffer
            },
            true => {
                let file = File::open(&self.file_name)?;
                let mut zip = (zip::ZipArchive::new(file))?;
                let mut f = zip.by_index(0).unwrap();
                if !f.name().to_lowercase().ends_with(".las") {
                    return Err(Error::new(ErrorKind::InvalidData,
                     "The data file contained within zipped archive does not have the proper 'las' extension."))
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
            },
        };

        self.header.project_id_used = true;
        self.header.version_major = buffer[24];
        self.header.version_minor = buffer[25];
        if self.header.version_major < 1 || self.header.version_major > 2 || self.header.version_minor > 5 {
            // There's something wrong. It could be that the project ID values, which are optional,
            // are not included in the header.
            self.header.version_major = buffer[8];
            self.header.version_minor = buffer[9];
            if self.header.version_major < 1 || self.header.version_major > 2 || self.header.version_minor > 5 {
                // There's something very wrong. Throw an error.
                return Err(Error::new(ErrorKind::Other, format!("Error reading {}\n. Either the file is formatted incorrectly or it is an unsupported LAS version.", self.file_name)));
            }
            self.header.project_id_used = false;
        }

        let mut bor = ByteOrderReader::new(buffer, Endianness::LittleEndian);
        
        bor.pos = 0;
        self.header.file_signature = bor.read_utf8(4);
        if self.header.file_signature != "LASF" {
            return Err(Error::new(ErrorKind::Other, format!("Error reading {}\n. Either the file is formatted incorrectly or it is an unsupported LAS version.", self.file_name)));
        }
        self.header.file_source_id = bor.read_u16();
        let ge_val = bor.read_u16();
        self.header.global_encoding = GlobalEncodingField { value: ge_val};
        if self.header.project_id_used {
            self.header.project_id1 = bor.read_u32();
            self.header.project_id2 = bor.read_u16();
            self.header.project_id3 = bor.read_u16();
            for i in 0..8 {
                self.header.project_id4[i] = bor.read_u8();
            }
        }
        // The version major and minor are read earlier.
        // Two bytes that must be added to the offset here.
        bor.pos += 2;
        self.header.system_id = bor.read_utf8(32); 
        self.header.generating_software = bor.read_utf8(32);
        self.header.file_creation_day = bor.read_u16();
        self.header.file_creation_year = bor.read_u16();
        self.header.header_size = bor.read_u16();
        self.header.offset_to_points = bor.read_u32();
        self.header.number_of_vlrs = bor.read_u32();
        self.header.point_format = bor.read_u8();
        self.header.point_record_length = bor.read_u16();
        self.header.number_of_points_old = bor.read_u32();

        for i in 0..5 {
            self.header.number_of_points_by_return_old[i] = bor.read_u32();
        }
        self.header.x_scale_factor = bor.read_f64();
        self.header.y_scale_factor = bor.read_f64();
        self.header.z_scale_factor = bor.read_f64();
        self.header.x_offset = bor.read_f64();
        self.header.y_offset = bor.read_f64();
        self.header.z_offset = bor.read_f64();
        self.header.max_x = bor.read_f64();
        self.header.min_x = bor.read_f64();
        self.header.max_y = bor.read_f64();
        self.header.min_y = bor.read_f64();
        self.header.max_z = bor.read_f64();
        self.header.min_z = bor.read_f64();

        if self.header.version_major == 1 && self.header.version_minor >= 3 {
            self.header.waveform_data_start = bor.read_u64();
            self.header.offset_to_ex_vlrs = bor.read_u64();
            self.header.number_of_extended_vlrs = bor.read_u32();
            self.header.number_of_points = bor.read_u64();
            for i in 0..15 {
                self.header.number_of_points_by_return[i] = bor.read_u64();
            }
        }

        if self.header.number_of_points_old != 0 { 
            self.header.number_of_points = self.header.number_of_points_old as u64;
            for i in 0..5 {
                if self.header.number_of_points_by_return_old[i] as u64 > self.header.number_of_points_by_return[i] {
                    self.header.number_of_points_by_return[i] = self.header.number_of_points_by_return_old[i] as u64;
                }
            }
        }

        ///////////////////////
        // Read the VLR data //
        ///////////////////////
        bor.seek(self.header.header_size as usize);
        for _ in 0..self.header.number_of_vlrs {
            let mut vlr: Vlr = Default::default();
            vlr.reserved = bor.read_u16();
            vlr.user_id = bor.read_utf8(16);
            vlr.record_id = bor.read_u16();
            vlr.record_length_after_header = bor.read_u16();
            vlr.description = bor.read_utf8(32);
            // get the byte data
            for _ in 0..vlr.record_length_after_header {
                vlr.binary_data.push(bor.read_u8());
            }
            
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
            
            // Intensity and userdata are both optional. Figure out if they need to be read.
            // The only way to do this is to compare the point record length by point format
            let rec_lengths = [ [20_u16, 18_u16, 19_u16, 17_u16],
                                [28_u16, 26_u16, 27_u16, 25_u16],
                                [26_u16, 24_u16, 25_u16, 23_u16],
                                [34_u16, 32_u16, 33_u16, 31_u16],
                                [57_u16, 55_u16, 56_u16, 54_u16],
                                [63_u16, 61_u16, 62_u16, 60_u16],
                                [30_u16, 28_u16, 29_u16, 27_u16],
                                [36_u16, 34_u16, 35_u16, 33_u16],
                                [38_u16, 36_u16, 37_u16, 35_u16],
                                [59_u16, 57_u16, 58_u16, 56_u16],
                                [67_u16, 65_u16, 66_u16, 64_u16] ];

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

            if self.header.point_format == 0 {
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.scan_angle = bor.read_i8() as i16;
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                }
            } else if self.header.point_format == 1 {
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.scan_angle = bor.read_i8() as i16;
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                }
            } else if self.header.point_format == 2 {
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.scan_angle = bor.read_i8() as i16;
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the RGB data
                    let mut rgb: ColourData = Default::default();
                    rgb.red = bor.read_u16();
                    rgb.green = bor.read_u16();
                    rgb.blue = bor.read_u16();
                    self.colour_data.push(rgb);
                }
            } else if self.header.point_format == 3 {
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.scan_angle = bor.read_i8() as i16;
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                    // read the RGB data
                    let mut rgb: ColourData = Default::default();
                    rgb.red = bor.read_u16();
                    rgb.green = bor.read_u16();
                    rgb.blue = bor.read_u16();
                    self.colour_data.push(rgb);
                }
            } else if self.header.point_format == 4 {
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.scan_angle = bor.read_i8() as i16;
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                    // read the waveform data
                    let mut wfp: WaveformPacket = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8();
                    wfp.offset_to_waveform_data = bor.read_u64();
                    wfp.waveform_packet_size = bor.read_u32();
                    wfp.ret_point_waveform_loc = bor.read_f32();
                    wfp.xt = bor.read_f32();
                    wfp.yt = bor.read_f32();
                    wfp.zt = bor.read_f32();
                    self.waveform_data.push(wfp);
                }
            } else if self.header.point_format == 5 {
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.scan_angle = bor.read_i8() as i16;
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                    // read the RGB data
                    let mut rgb: ColourData = Default::default();
                    rgb.red = bor.read_u16();
                    rgb.green = bor.read_u16();
                    rgb.blue = bor.read_u16();
                    self.colour_data.push(rgb);
                    // read the waveform data
                    let mut wfp: WaveformPacket = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8();
                    wfp.offset_to_waveform_data = bor.read_u64();
                    wfp.waveform_packet_size = bor.read_u32();
                    wfp.ret_point_waveform_loc = bor.read_f32();
                    wfp.xt = bor.read_f32();
                    wfp.yt = bor.read_f32();
                    wfp.zt = bor.read_f32();
                    self.waveform_data.push(wfp);
                }
            } else if self.header.point_format == 6 { // 64-bit
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.is_64bit = true;
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.classification = bor.read_u8();
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.scan_angle = bor.read_i16();
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                }
            } else if self.header.point_format == 7 { // 64-bit
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.is_64bit = true;
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.classification = bor.read_u8();
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.scan_angle = bor.read_i16();
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                    // read the RGB data
                    let mut rgb: ColourData = Default::default();
                    rgb.red = bor.read_u16();
                    rgb.green = bor.read_u16();
                    rgb.blue = bor.read_u16();
                    self.colour_data.push(rgb);
                }
            } else if self.header.point_format == 8 { // 64-bit
                // adds a NIR band to Point Format 7
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.is_64bit = true;
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.classification = bor.read_u8();
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.scan_angle = bor.read_i16();
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                    // read the RGBNIR data
                    let mut rgb: ColourData = Default::default();
                    rgb.red = bor.read_u16();
                    rgb.green = bor.read_u16();
                    rgb.blue = bor.read_u16();
                    rgb.nir = bor.read_u16();
                    self.colour_data.push(rgb);
                }
            } else if self.header.point_format == 9 { // 64-bit
                // adds waveform packets to Point Format 6
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.is_64bit = true;
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.classification = bor.read_u8();
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.scan_angle = bor.read_i16();
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                    // read the waveform data
                    let mut wfp: WaveformPacket = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8();
                    wfp.offset_to_waveform_data = bor.read_u64();
                    wfp.waveform_packet_size = bor.read_u32();
                    wfp.ret_point_waveform_loc = bor.read_f32();
                    wfp.xt = bor.read_f32();
                    wfp.yt = bor.read_f32();
                    wfp.zt = bor.read_f32();
                    self.waveform_data.push(wfp);
                }
            } else if self.header.point_format == 10 { // 64-bit
                // Everythin in one record
                for i in 0..self.header.number_of_points {
                    bor.seek(self.header.offset_to_points as usize + (i as usize) * (self.header.point_record_length as usize));
                    let mut p: PointData = Default::default();
                    p.is_64bit = true;
                    p.x = bor.read_i32() as f64 * self.header.x_scale_factor + self.header.x_offset;
                    p.y = bor.read_i32() as f64 * self.header.y_scale_factor + self.header.y_offset;
                    p.z = bor.read_i32() as f64 * self.header.z_scale_factor + self.header.z_offset;
                    if self.use_point_intensity { p.intensity = bor.read_u16(); }
                    p.point_bit_field = bor.read_u8();
                    p.class_bit_field = bor.read_u8();
                    p.classification = bor.read_u8();
                    if self.use_point_userdata { p.user_data = bor.read_u8(); }
                    p.scan_angle = bor.read_i16();
                    p.point_source_id = bor.read_u16();
                    self.point_data.push(p);
                    // read the GPS data
                    self.gps_data.push(bor.read_f64());
                    // read the RGBNIR data
                    let mut rgb: ColourData = Default::default();
                    rgb.red = bor.read_u16();
                    rgb.green = bor.read_u16();
                    rgb.blue = bor.read_u16();
                    rgb.nir = bor.read_u16();
                    self.colour_data.push(rgb);
                    // read the waveform data
                    let mut wfp: WaveformPacket = Default::default();
                    wfp.packet_descriptor_index = bor.read_u8();
                    wfp.offset_to_waveform_data = bor.read_u64();
                    wfp.waveform_packet_size = bor.read_u32();
                    wfp.ret_point_waveform_loc = bor.read_f32();
                    wfp.xt = bor.read_f32();
                    wfp.yt = bor.read_f32();
                    wfp.zt = bor.read_f32();
                    self.waveform_data.push(wfp);
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

        if !self.file_name.to_lowercase().ends_with(".zip") {
            let f = File::create(&self.file_name)?;
            let mut writer = BufWriter::new(f);

            self.write_data(&mut writer)?;
        
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
        
        u16_bytes = unsafe {mem::transmute(self.header.file_source_id)};
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
        let mut u8_bytes: [u8; 1] = unsafe {mem::transmute(self.header.version_major)};
        writer.write_all(&u8_bytes)?;
        
        self.header.version_minor = 3u8;
        u8_bytes = unsafe {mem::transmute(self.header.version_minor)};
        writer.write_all(&u8_bytes)?;
        
        if self.header.system_id.len() == 0 {
            self.header.system_id = fixed_length_string("OTHER", 32);
        } else if !self.header.system_id.len() != 32 {
            self.header.system_id = fixed_length_string(&(self.header.system_id), 32);
        }
        writer.write_all(self.header.system_id.as_bytes())?; //string_bytes));
        
        self.header.generating_software = fixed_length_string("WhiteboxTools                   ", 32);
        writer.write_all(self.header.generating_software.as_bytes())?;
        
        let now = time::now();
        self.header.file_creation_day = now.tm_yday as u16;
        u16_bytes = unsafe { mem::transmute(self.header.file_creation_day) };
        writer.write_all(&u16_bytes)?;
        
        self.header.file_creation_year = (now.tm_year + 1900) as u16;
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
        self.header.offset_to_points = 235 + total_vlr_size; // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
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
                println!("Warning: Point Format 4 is not supported for output. Some data will be lost."); 
                1u8
            }, 
            5u8 => { 
                println!("Warning: Point Format 5 is not supported for output. Some data will be lost."); 
                3u8
            }, 
            6u8 => 1u8,
            7u8 => 3u8,
            8u8 => { 
                println!("Warning: Point Format 8 is not supported for output. Some data will be lost."); 
                3u8
            },  
            9u8 => { 
                println!("Warning: Point Format 9 is not supported for output. Some data will be lost."); 
                1u8
            }, 
            10u8 => { 
                println!("Warning: Point Format 10 is not supported for output. Some data will be lost."); 
                3u8
            },  
            _ => { 
                return Err(Error::new(ErrorKind::Other, "Unsupported point format")); 
            },
        };
        
        u8_bytes = unsafe {mem::transmute(self.header.point_format)};
        writer.write_all(&u8_bytes)?;

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
        writer.write_all(&u16_bytes)?;
        
        if self.header.number_of_points <= u32::max_value() as u64 {
            self.header.number_of_points_old = self.header.number_of_points as u32; // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
        } else {
            return Err(Error::new(ErrorKind::Other, "The number of points in this file requires a 64-bit format. Currently LAS 1.4 files cannot be written.")); 
        }
        u32_bytes = unsafe { mem::transmute(self.header.number_of_points_old) };
        writer.write_all(&u32_bytes)?;
        
        for i in 0..5 { // THIS NEEDS TO BE FIXED WHEN LAS 1.4 SUPPORT IS ADDED FOR WRITING
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

        ////////////////////////////////
        // Write the point to the file /
        ////////////////////////////////
        let mut val: i32;
        match self.header.point_format {
            0 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].point_bit_field)};
                    writer.write_all(&u8_bytes)?;
                    
                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field)};
                    writer.write_all(&u8_bytes)?;
                    
                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle as i8)};
                    writer.write_all(&u8_bytes)?;
                    
                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
                        writer.write_all(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write_all(&u16_bytes)?;
                }
            },
            1 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].point_bit_field)};
                    writer.write_all(&u8_bytes)?;
                    
                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field)};
                    writer.write_all(&u8_bytes)?;
                    
                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle as i8)};
                    writer.write_all(&u8_bytes)?;
                    
                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
                        writer.write_all(&u8_bytes)?;
                    }

                    u16_bytes = unsafe { mem::transmute(self.point_data[i].point_source_id) };
                    writer.write_all(&u16_bytes)?;
                    
                    u64_bytes = unsafe { mem::transmute(self.gps_data[i]) };
                    writer.write_all(&u64_bytes)?;
                }
            },
            2 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].point_bit_field)};
                    writer.write_all(&u8_bytes)?;

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field)};
                    writer.write_all(&u8_bytes)?;
                    
                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle as i8)};
                    writer.write_all(&u8_bytes)?;
                    
                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
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
            },
            3 => {
                for i in 0..self.header.number_of_points as usize {
                    val = ((self.point_data[i].x - self.header.x_offset) / self.header.x_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].y - self.header.y_offset) / self.header.y_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    val = ((self.point_data[i].z - self.header.z_offset) / self.header.z_scale_factor) as i32;
                    u32_bytes = unsafe { mem::transmute(val) };
                    writer.write_all(&u32_bytes)?;
                    
                    if self.use_point_intensity {
                        u16_bytes = unsafe { mem::transmute(self.point_data[i].intensity) };
                        writer.write_all(&u16_bytes)?;
                    }

                    u8_bytes = unsafe {mem::transmute(self.point_data[i].point_bit_field)};
                    writer.write_all(&u8_bytes)?;
                    
                    u8_bytes = unsafe {mem::transmute(self.point_data[i].class_bit_field)};
                    writer.write_all(&u8_bytes)?;
                    
                    u8_bytes = unsafe {mem::transmute(self.point_data[i].scan_angle as i8)};
                    writer.write_all(&u8_bytes)?;
                    
                    if self.use_point_userdata {
                        u8_bytes = unsafe {mem::transmute(self.point_data[i].user_data)};
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
    PointRecord2 { point_data: PointData, colour_data: ColourData },
    PointRecord3 { point_data: PointData, gps_data: f64, colour_data: ColourData },
    PointRecord4 { point_data: PointData, gps_data: f64, wave_packet: WaveformPacket },
    PointRecord5 { point_data: PointData, gps_data: f64, colour_data: ColourData, wave_packet: WaveformPacket },
    PointRecord6 { point_data: PointData, gps_data: f64 },
    PointRecord7 { point_data: PointData, gps_data: f64, colour_data: ColourData },
    PointRecord8 { point_data: PointData, gps_data: f64, colour_data: ColourData },
    PointRecord9 { point_data: PointData, gps_data: f64, wave_packet: WaveformPacket },
    PointRecord10 { point_data: PointData, gps_data: f64, colour_data: ColourData, wave_packet: WaveformPacket }
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

fn browse_zip_archive<T, F, U>(buf: &mut T, browse_func: F) -> ZipResult<Vec<U>> where T: Read + Seek, F: Fn(&ZipFile) -> ZipResult<U> {
    let mut archive = ZipArchive::new(buf)?;
    (0..archive.len())
        .map(|i| archive.by_index(i).and_then(|file| browse_func(&file)))
        .collect()
}
