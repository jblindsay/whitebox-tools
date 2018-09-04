/* 
This file is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/04/2018
Last Modified: 12/04/2018
License: MIT

NOTE: Structures and functions for handling the Shapefile attribute table info 
contained with the associated .dbf file.
*/

use std::collections::HashMap;
use std::fmt;

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

impl fmt::Display for DateData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut m = self.month.to_string();
        if m.len() < 2 {
            m = format!("0{}", m);
        }
        if m.len() > 2 {
            m = m[m.len() - 2..m.len()].to_string();
        }
        let mut d = self.day.to_string();
        if d.len() < 2 {
            d = format!("0{}", d);
        }
        if d.len() > 2 {
            d = d[d.len() - 2..d.len()].to_string();
        }
        let s = format!("{}{}{}", self.year, m, d);
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub enum FieldData {
    Int(i32),
    // Int64(i64),
    Real(f64),
    Text(String),
    Date(DateData),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum FieldDataType {
    Int,
    Real,
    Text,
    Date,
    Bool,
}

impl FieldDataType {
    pub fn to_char(&self) -> char {
        let c = match *self {
            FieldDataType::Int => 'N',
            FieldDataType::Real => 'F',
            FieldDataType::Text => 'C',
            FieldDataType::Date => 'D',
            FieldDataType::Bool => 'L',
        };
        c
    }
}

#[derive(Debug, Default, Clone)]
pub struct AttributeField {
    pub name: String,
    pub field_type: char,
    pub field_length: u8,
    pub decimal_count: u8,
    // pub work_area_id: u8,
    // pub set_field_flag: u8,
    // pub index_field_flag: u8,
}

impl AttributeField {
    pub fn new<'a>(
        name: &'a str,
        field_type: FieldDataType,
        field_length: u8,
        decimal_count: u8,
        // work_area_id: u8,
        // set_field_flag: u8,
        // index_field_flag: u8,
    ) -> AttributeField {
        AttributeField {
            name: name.to_string(),
            field_type: field_type.to_char(),
            field_length: field_length,
            decimal_count: decimal_count,
            // work_area_id: work_area_id,
            // set_field_flag: set_field_flag,
            // index_field_flag: index_field_flag,
        }
    }
}

#[derive(Default, Clone)]
pub struct ShapefileAttributes {
    pub header: AttributeHeader,
    pub fields: Vec<AttributeField>,
    data: Vec<Vec<FieldData>>,
    pub is_deleted: Vec<bool>,
    field_map: HashMap<String, usize>,
}

impl ShapefileAttributes {
    /// Adds a field to the table
    pub fn add_field<'a>(&mut self, field: &'a AttributeField) {
        self.fields.push(field.clone());
        self.header.num_fields += 1;
        self.get_field_hashmap();
        // println!("{}", field.name);
        for record_index in 0..self.data.len() {
            self.data[record_index].push(FieldData::Null);
            // println!("{:?}", self.data[record_index]);
        }
        // println!(
        //     "{} {} {}",
        //     self.header.num_records,
        //     self.header.num_fields,
        //     self.data.len(),
        // );
    }

    /// Adds a Vec of fields to the table
    pub fn add_fields<'a>(&mut self, fields: &'a Vec<AttributeField>) {
        for field in fields {
            self.fields.push(field.clone());
            self.header.num_fields += 1;
        }
        for record_index in 0..self.data.len() {
            for _ in 0..fields.len() {
                self.data[record_index].push(FieldData::Null);
            }
        }
        self.get_field_hashmap();
    }

    /// Returns a field from the table
    pub fn get_field<'a>(&'a self, index: usize) -> &'a AttributeField {
        &self.fields[index]
    }

    /// Returns the fields of a table
    pub fn get_fields<'a>(&'a self) -> &'a Vec<AttributeField> {
        &self.fields
    }

    /// Adds an attribute record to the table.
    pub fn add_record(&mut self, rec: Vec<FieldData>, deleted: bool) {
        self.data.push(rec);
        self.is_deleted.push(deleted);
        self.header.num_records = self.data.len() as u32; //+= 1;
    }

    /// Retrieves an attribute record for a zero-based index. The returned data is a copy of the original.
    pub fn get_record(&self, index: usize) -> Vec<FieldData> {
        if index >= self.header.num_records as usize {
            panic!("Error: Specified record index is greater than the number of records.");
        }
        self.data[index].clone()
    }

    pub fn get_value(&self, record_index: usize, field_name: &str) -> FieldData {
        if record_index >= self.header.num_records as usize {
            panic!("Error: Specified record index is greater than the number of records.");
        }
        let field_index = self.field_map[field_name];
        if field_index >= self.fields.len() {
            panic!("Error: Specified field does not appear in attribute table.");
        }
        self.data[record_index][field_index].clone()
    }

    pub fn set_value(&mut self, record_index: usize, field_name: &str, field_data: FieldData) {
        if record_index >= self.header.num_records as usize {
            panic!("Error: Specified record index is greater than the number of records.");
        }
        let field_index = self.field_map[field_name];
        if field_index >= self.fields.len() {
            panic!("Error: Specified field does not appear in attribute table.");
        }
        self.data[record_index][field_index] = field_data.clone();
    }

    // pub fn get_field_value(&self, record_index: usize, field_index: usize) -> FieldData {
    //     self.data[record_index][field_index].clone()
    // }

    pub fn get_field_num(&self, name: &str) -> Option<usize> {
        for i in 0..self.fields.len() {
            if self.fields[i].name == name {
                return Some(i);
            }
        }
        None
    }

    fn get_field_hashmap(&mut self) {
        self.field_map.clear();
        for i in 0..self.fields.len() {
            self.field_map.insert(self.fields[i].name.clone(), i);
        }
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
            'N' | 'F' | 'I' | 'O' => return true,
            _ => return false,
        }
    }
}
