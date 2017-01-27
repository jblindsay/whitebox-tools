use std::fmt;
use lidar::las::GlobalEncodingField;

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
    pub point_format: u8,
    pub point_record_length: u16,
    pub number_of_points: u32,
    pub number_of_points_by_return: [u32; 5],
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
            s = s + &format!("\nProject ID (GUID): {{{:x}-{:x}-{:x}-{:x}{:x}-{:x}{:x}{:x}{:x}{:x}{:x}}}",
                self.project_id1, self.project_id2, self.project_id3, self.project_id4[0],
                self.project_id4[1], self.project_id4[2], self.project_id4[3], self.project_id4[4],
                self.project_id4[5], self.project_id4[6], self.project_id4[7]);
        }
        s = s + &format!("\nSystem ID: {}", self.system_id);
        s = s + &format!("\nGenerating Software: {}", self.generating_software);
        s = s + &format!("\nLas Version: {}.{}", self.version_major, self.version_minor);
        s = s + &format!("\nFile Creation Day/Year: {}/{}", self.file_creation_day, self.file_creation_year);
        s = s + &format!("\nHeader Size: {}", self.header_size);
        s = s + &format!("\nOffset to Points: {}", self.offset_to_points);
        s = s + &format!("\nNumber of VLRs: {}", self.number_of_vlrs);
        s = s + &format!("\nPoint Format: {}", self.point_format);
        s = s + &format!("\nPoint Record Length: {}", self.point_record_length);
        s = s + &format!("\nNum. of Points: {}", self.number_of_points);
        s = s + &"\nNumber of Points by Return: [";
        for i in 0..self.number_of_points_by_return.len() {
            if i < self.number_of_points_by_return.len()-1 {
                s = s + &format!("{}, ", self.number_of_points_by_return[i]);
            } else {
                s = s + &format!("{}]", self.number_of_points_by_return[i]);
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

        write!(f, "{}", s)
    }
}
