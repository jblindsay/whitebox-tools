use super::las::GlobalEncodingField;
use crate::utils::{ByteOrderReader, Endianness};
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};

#[derive(Default, Clone, Debug)]
pub struct LasHeader {
    pub file_signature: String,
    pub file_source_id: u16,
    pub global_encoding: GlobalEncodingField,
    pub project_id_used: bool,
    pub project_id1: u32,
    pub project_id2: u16,
    pub project_id3: u16,
    pub project_id4: [u8; 8],
    pub version_major: u8,
    pub version_minor: u8,
    pub system_id: String,
    pub generating_software: String,
    pub file_creation_day: u16,
    pub file_creation_year: u16,
    pub header_size: u16,
    pub offset_to_points: u32,
    pub number_of_vlrs: u32,
    pub number_of_extended_vlrs: u32,
    pub offset_to_ex_vlrs: u64,
    pub point_format: u8,
    pub point_record_length: u16,
    pub number_of_points_old: u32,
    pub number_of_points: u64,
    pub number_of_points_by_return_old: [u32; 5],
    pub number_of_points_by_return: [u64; 15],
    pub x_scale_factor: f64,
    pub y_scale_factor: f64,
    pub z_scale_factor: f64,
    pub x_offset: f64,
    pub y_offset: f64,
    pub z_offset: f64,
    pub max_x: f64,
    pub min_x: f64,
    pub max_y: f64,
    pub min_y: f64,
    pub max_z: f64,
    pub min_z: f64,
    pub waveform_data_start: u64,
}

impl fmt::Display for LasHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = format!("\nFile Signature: {}", self.file_signature);
        s = s + &format!("\nFile Source ID: {}", self.file_source_id);
        s = s + &format!("\nGlobal Encoding:\n{}", self.global_encoding);
        if self.project_id_used {
            s = s + &format!(
                "\nProject ID (GUID): {{{:x}-{:x}-{:x}-{:x}{:x}-{:x}{:x}{:x}{:x}{:x}{:x}}}",
                self.project_id1,
                self.project_id2,
                self.project_id3,
                self.project_id4[0],
                self.project_id4[1],
                self.project_id4[2],
                self.project_id4[3],
                self.project_id4[4],
                self.project_id4[5],
                self.project_id4[6],
                self.project_id4[7]
            );
        }
        s = s + &format!("\nSystem ID: {}", self.system_id);
        s = s + &format!("\nGenerating Software: {}", self.generating_software);
        s = s + &format!(
            "\nLas Version: {}.{}",
            self.version_major, self.version_minor
        );
        s = s + &format!(
            "\nFile Creation Day/Year: {}/{}",
            self.file_creation_day, self.file_creation_year
        );
        s = s + &format!("\nHeader Size: {}", self.header_size);
        s = s + &format!("\nOffset to Points: {}", self.offset_to_points);
        s = s + &format!("\nNumber of VLRs: {}", self.number_of_vlrs);
        s = s + &format!("\nPoint Format: {}", self.point_format);
        s = s + &format!("\nPoint Record Length: {}", self.point_record_length);
        s = s + &format!("\nNum. of Points (32-bit): {}", self.number_of_points_old);
        s = s + &"\nNumber of Points by Return: [";
        for i in 0..self.number_of_points_by_return_old.len() {
            if i < self.number_of_points_by_return_old.len() - 1 {
                s = s + &format!("{}, ", self.number_of_points_by_return_old[i]);
            } else {
                s = s + &format!("{}]", self.number_of_points_by_return_old[i]);
            }
        }
        s = s + &format!("\nX Scale Factor: {}", self.x_scale_factor);
        s = s + &format!("\nY Scale Factor: {}", self.y_scale_factor);
        s = s + &format!("\nZ Scale Factor: {}", self.z_scale_factor);
        s = s + &format!("\nX Offset: {}", self.x_offset);
        s = s + &format!("\nY Offset: {}", self.y_offset);
        s = s + &format!("\nZ Offset: {}", self.z_offset);

        s = s + &format!("\nMax X: {}", self.max_x);
        s = s + &format!("\nMin X: {}", self.min_x);
        s = s + &format!("\nMax Y: {}", self.max_y);
        s = s + &format!("\nMin Y: {}", self.min_y);
        s = s + &format!("\nMax Z: {}", self.max_z);
        s = s + &format!("\nMin Z: {}", self.min_z);

        s = s + &format!("\nWaveform Data Start: {}", self.waveform_data_start);

        if self.version_minor >= 4 {
            s = s + &format!("\nExtended VLR Start: {}", self.offset_to_ex_vlrs);
            s = s + &format!("\nNum. Extended VLR: {}", self.number_of_extended_vlrs);
            s = s + &format!("\nNum. of Points (64-bit): {}", self.number_of_points);
            s = s + &"\nNumber of Points by Return (64-bit): [";
            for i in 0..self.number_of_points_by_return.len() {
                if i < self.number_of_points_by_return.len() - 1 {
                    s = s + &format!("{}, ", self.number_of_points_by_return[i]);
                } else {
                    s = s + &format!("{}]", self.number_of_points_by_return[i]);
                }
            }
        }

        write!(f, "{}", s)
    }
}

impl LasHeader {
    /*
    This function can be used when you just want the metadata contained in a LAS file's
    header but don't want to read the file's data.
    */
    pub fn read_las_header(file_name: &str) -> Result<LasHeader, Error> {
        let mut f = File::open(file_name)?;
        let mut buffer = vec![0; 375]; // A LAS header is about 375 bytes, depending on optional parameters.

        // read the file's header bytes into a buffer
        f.read(&mut buffer)?;
        let mut header: LasHeader = Default::default();

        header.project_id_used = true;
        header.version_major = buffer[24];
        header.version_minor = buffer[25];
        if header.version_major < 1 || header.version_major > 2 || header.version_minor > 5 {
            // There's something wrong. It could be that the project ID values, which are optional,
            // are not included in the header.
            header.version_major = buffer[8];
            header.version_minor = buffer[9];
            if header.version_major < 1 || header.version_major > 2 || header.version_minor > 5 {
                // There's something very wrong. Throw an error.
                return Err(Error::new(ErrorKind::Other, format!("Error reading {}\n. Either the file is formatted incorrectly or it is an unsupported LAS version.", file_name)));
            }
            header.project_id_used = false;
        }

        let mut bor = ByteOrderReader::new(buffer, Endianness::LittleEndian);

        bor.pos = 0;
        header.file_signature = bor.read_utf8(4);
        if header.file_signature != "LASF" {
            return Err(Error::new(ErrorKind::Other, format!("Error reading {}\n. Either the file is formatted incorrectly or it is an unsupported LAS version.", file_name)));
        }
        header.file_source_id = bor.read_u16();
        let ge_val = bor.read_u16();
        header.global_encoding = GlobalEncodingField { value: ge_val };
        if header.project_id_used {
            header.project_id1 = bor.read_u32();
            header.project_id2 = bor.read_u16();
            header.project_id3 = bor.read_u16();
            for i in 0..8 {
                header.project_id4[i] = bor.read_u8();
            }
        }
        // The version major and minor are read earlier.
        // Two bytes that must be added to the offset here.
        bor.pos += 2;
        header.system_id = bor.read_utf8(32);
        header.generating_software = bor.read_utf8(32);
        header.file_creation_day = bor.read_u16();
        header.file_creation_year = bor.read_u16();
        header.header_size = bor.read_u16();
        header.offset_to_points = bor.read_u32();
        header.number_of_vlrs = bor.read_u32();
        header.point_format = bor.read_u8();
        header.point_record_length = bor.read_u16();
        header.number_of_points_old = bor.read_u32();

        for i in 0..5 {
            header.number_of_points_by_return_old[i] = bor.read_u32();
        }
        header.x_scale_factor = bor.read_f64();
        header.y_scale_factor = bor.read_f64();
        header.z_scale_factor = bor.read_f64();
        header.x_offset = bor.read_f64();
        header.y_offset = bor.read_f64();
        header.z_offset = bor.read_f64();
        header.max_x = bor.read_f64();
        header.min_x = bor.read_f64();
        header.max_y = bor.read_f64();
        header.min_y = bor.read_f64();
        header.max_z = bor.read_f64();
        header.min_z = bor.read_f64();

        if header.version_major == 1 && header.version_minor >= 3 {
            header.waveform_data_start = bor.read_u64();
            header.offset_to_ex_vlrs = bor.read_u64();
            header.number_of_extended_vlrs = bor.read_u32();
            header.number_of_points = bor.read_u64();
            for i in 0..15 {
                header.number_of_points_by_return[i] = bor.read_u64();
            }
        }

        if header.number_of_points_old != 0 {
            header.number_of_points = header.number_of_points_old as u64;
            for i in 0..5 {
                if header.number_of_points_by_return_old[i] as u64
                    > header.number_of_points_by_return[i]
                {
                    header.number_of_points_by_return[i] =
                        header.number_of_points_by_return_old[i] as u64;
                }
            }
        }

        Ok(header)
    }
}
