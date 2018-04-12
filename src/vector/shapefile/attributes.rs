/* 
This file is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/04/2018
Last Modified: 12/04/2018
License: MIT

NOTE: Structures and functions for handling the Shapefile attribute table info 
contained with the associated .dbf file.
*/

#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Copy, Clone)]
pub struct DateData {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone)]
pub enum FieldData {
    Int(i32),
    Int64(i64),
    Real(f64),
    Text(String),
    Date(DateData),
    Bool(bool),
    Null,
}


#[derive(Debug, Default, Clone)]
pub struct AttributeField {
    pub name: String,
    pub field_type: char,
    pub field_length: u8,
    pub decimal_count: u8,
    work_area_id: u8,
    set_field_flag: u8,
    index_field_flag: u8,
}

impl AttributeField {
    pub fn new<'a>(name: &str, 
        field_type: char, 
        field_length: u8, 
        decimal_count: u8,
        work_area_id: u8, 
        set_field_flag: u8, 
        index_field_flag: u8) -> AttributeField {
        
        AttributeField {
            name: name.to_string(),
            field_type: field_type,
            field_length: field_length,
            decimal_count: decimal_count,
            work_area_id: work_area_id,
            set_field_flag: set_field_flag,
            index_field_flag: index_field_flag,
        }
    }
}

#[derive(Default, Clone)]
pub struct ShapefileAttributes {
    pub header: AttributeHeader,
    pub fields: Vec<AttributeField>,
    data: Vec<Vec<FieldData>>,
    deleted: Vec<bool>,
}

impl ShapefileAttributes {
    pub fn add_record(&mut self, deleted: bool, rec: Vec<FieldData>) {
        self.data.push(rec);
        self.deleted.push(deleted);
    }

    pub fn get_record(&self, index: usize) -> Vec<FieldData> {
        if index >= self.header.num_records as usize {
            panic!("Error: Specified record index is greater than the number of records.");
        }
        // if self.deleted[index] {
            
        // }
        self.data[index].clone()
    }

    pub fn get_field_value(&self, record_index: usize, field_index: usize) -> FieldData {
        self.data[record_index][field_index].clone()
    }

    pub fn get_field_num(&self, name: &str) -> Option<usize> {
        for i in 0..self.fields.len() {
            if self.fields[i].name == name {
                return Some(i)
            }
        }
        None 
    }

    pub fn get_field_info(&self, index: usize) -> AttributeField {
        if index >= self.header.num_records as usize {
            panic!("Error: Specified field is greater than the number of fields.");
        }
        self.fields[index].clone()
    }

    pub fn is_field_numeric(&self, index: usize) -> bool {
        if index >= self.header.num_records as usize {
            panic!("Error: Specified field is greater than the number of fields.");
        }
        match self.fields[index].field_type {
            'N' | 'F' => return true,
            _ => return false,
        }
    }
}