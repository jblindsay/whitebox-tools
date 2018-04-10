/* 
This file is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/04/2018
Last Modified: 10/04/2018
License: MIT

NOTE: Structures and functions for handling the Shapefile attribute table info 
contained with the associated .dbf file.
*/

#[derive(Default, Clone)]
pub struct AttributeHeader {
    pub version: u8,
    pub year: u32,
    pub month: u8,
    pub day: u8,
    pub num_records: u32,
    pub num_fields: u32, // not actually stored in file but derived
    pub bytes_in_header: u16,
    pub bytes_in_record: u16,
    pub incomplete_tansaction: u8,
    pub encryption_flag: u8,
    pub mdx_flag: u8,
    pub language_driver_id: u8,
}

#[derive(Default, Clone)]
pub struct AttributeField {
    pub name: String,
    pub field_type: char,
    pub field_length: u8,
    pub decimal_count: u8,
    work_area_id: u8,
    set_field_flag: u8,
    index_field_flag: u8,
}

#[derive(Default, Clone)]
pub struct ShapefileAttributes {
    pub header: AttributeHeader,
    pub fields: Vec<AttributeField>,
    data: Vec<Vec<String>>,
    deleted: Vec<bool>,
}
