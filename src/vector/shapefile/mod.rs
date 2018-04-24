/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: 12/04/2018
License: MIT

Notes: The logic behind working with the ESRI Shapefile format.
*/
pub mod attributes;
pub mod geometry;

pub use self::attributes::{AttributeField, AttributeHeader, DateData, FieldData, ShapefileAttributes};
pub use self::geometry::{ShapefileGeometry, ShapeType};
use std::io::prelude::*;
use std::io::{BufReader, Error, ErrorKind};
use std::fs;
use std::fs::File;
use std::fmt;
use std::str;
use io_utils::{ByteOrderReader, Endianness};
use vector::Point2D;

// 100 bytes in size
#[derive(Default, Clone)]
pub struct ShapefileHeader {
    file_code: i32, // BigEndian; value is 9994
    unused1: i32, // BigEndian
    unused2: i32, // BigEndian
    unused3: i32, // BigEndian
    unused4: i32, // BigEndian
    unused5: i32, // BigEndian
    pub file_length: i32, // BigEndian
    pub version: i32, // LittleEndian
    pub shape_type: ShapeType, // LittleEndian
    pub x_min: f64, // LittleEndian
    pub y_min: f64, // LittleEndian
    pub x_max: f64, // LittleEndian
    pub y_max: f64, // LittleEndian
    pub z_min: f64, // LittleEndian; set to 0f64 if shapeType not z or measured
    pub z_max: f64, // LittleEndian; set to 0f64 if shapeType not z or measured
    pub m_min: f64, // LittleEndian; set to 0f64 if shapeType not z or measured
    pub m_max: f64, // LittleEndian; set to 0f64 if shapeType not z or measured
}

impl fmt::Display for ShapefileHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("file_code: {}
file_length: {}
version: {}
shape_type: {}
x_min: {}
x_max: {}
y_min: {}
y_max: {}
z_min: {}
z_max: {}
m_min: {}
m_max: {}", 
        self.file_code,
        self.file_length,
        self.version,
        self.shape_type,
        self.x_min,
        self.x_max,
        self.y_min,
        self.y_max,
        self.z_min,
        self.z_max,
        self.m_min,
        self.m_max);
        write!(f, "{}", s)
    }
}

#[derive(Default, Clone)]
pub struct Shapefile {
    pub file_name: String,
    pub file_mode: String,
    pub header: ShapefileHeader,
    pub num_records: usize,
    pub records: Vec<ShapefileGeometry>,
    pub attributes: ShapefileAttributes,
    pub projection: String,
}

impl Shapefile {
    pub fn new<'a>(file_name: &'a str, file_mode: &'a str) -> Result<Shapefile, Error> {
        let fm: String = file_mode.to_lowercase();
        let mut sf = Shapefile {
            file_name: file_name.to_string(),
            file_mode: fm.clone(),
            ..Default::default()
        };
        if sf.file_mode.contains("r") {
            sf.read()?;
        } else {
            // write
            // return Ok(r);
        }
        Ok(sf)
        
    }

    pub fn get_record<'a>(&'a self, index: usize) -> &'a ShapefileGeometry {
        if index >= self.records.len() { panic!("Record index out of bounds"); }
        &self.records[index]
    }

    fn read(&mut self) -> Result<(), Error>  {
        ///////////////////////////////
        // First read the geometries //
        ///////////////////////////////

        // read the header
        let mut f = File::open(self.file_name.clone()).unwrap(); //?;
        let metadata = fs::metadata(self.file_name.clone()).unwrap(); //?;
        let file_size: usize = metadata.len() as usize;
        let mut buffer = vec![0; file_size];

        // read the file's bytes into a buffer
        f.read(&mut buffer).unwrap(); //?;

        // Note: the shapefile format uses mixed endianness for whatever reason.
        // The ByteOrderReader was set up to have one consistent endianness. As
        // such, we will need to switch the endianness frequently.
        let mut bor = ByteOrderReader::new(buffer, Endianness::BigEndian);
        bor.seek(0);
        self.header.file_code = bor.read_i32();
        bor.seek(24);
        self.header.file_length = bor.read_i32();
        
        // the rest of the header is in LittleEndian format
        bor.byte_order = Endianness::LittleEndian;
        self.header.version = bor.read_i32();
        self.header.shape_type = ShapeType::from_int(bor.read_i32());
        
        // bounding box
        self.header.x_min = bor.read_f64();
        self.header.y_min = bor.read_f64();
        self.header.x_max = bor.read_f64();
        self.header.y_max = bor.read_f64();
        self.header.z_min = bor.read_f64();
        self.header.z_max = bor.read_f64();
        self.header.m_min = bor.read_f64();
        self.header.m_max = bor.read_f64();

        // Read the data
        bor.byte_order = Endianness::LittleEndian;
        match self.header.shape_type {
            ShapeType::Point => {
                while bor.pos < file_size {
                    bor.pos += 12; // Don't need to read the header and shapeType. It's not going to change.
                    let sfg = ShapefileGeometry {
                        shape_type: ShapeType::Point, 
                        num_points: 1i32, 
                        points: vec![ Point2D{ x: bor.read_f64(), y: bor.read_f64() } ],
                        ..Default::default()
                    };
                    self.records.push(sfg);
                }
            },

            ShapeType::PolyLine | ShapeType::Polygon => {
                while bor.pos < file_size {
                    bor.pos += 8;
                    let mut sfg = ShapefileGeometry {
                        shape_type: ShapeType::from_int(bor.read_i32()), 
                        x_min: bor.read_f64(),
                        y_min: bor.read_f64(),
                        x_max: bor.read_f64(),
                        y_max: bor.read_f64(),
                        num_parts: bor.read_i32(),
                        num_points: bor.read_i32(), 
                        ..Default::default()
                    };

                    for _ in 0..sfg.num_parts {
                        sfg.parts.push(bor.read_i32());
                    }

                    for _ in 0..sfg.num_points {
                        sfg.points.push(Point2D{ x: bor.read_f64(), y: bor.read_f64() });
                    }  

                    self.records.push(sfg);
                }
            },

            ShapeType::MultiPoint => {
                while bor.pos < file_size {
                    bor.pos += 8;
                    let mut sfg = ShapefileGeometry {
                        shape_type: ShapeType::from_int(bor.read_i32()), 
                        x_min: bor.read_f64(),
                        y_min: bor.read_f64(),
                        x_max: bor.read_f64(),
                        y_max: bor.read_f64(),
                        num_points: bor.read_i32(), 
                        ..Default::default()
                    };

                    for _ in 0..sfg.num_points {
                        sfg.points.push(Point2D{ x: bor.read_f64(), y: bor.read_f64() });
                    }  

                    self.records.push(sfg);
                }
            },

            ShapeType::PointZ => {
                while bor.pos < file_size {
                    bor.pos += 12; // Don't need to read the header and shapeType. It's not going to change.
                    let sfg = ShapefileGeometry {
                        shape_type: ShapeType::Point, 
                        num_points: 1i32, 
                        points: vec![ Point2D{ x: bor.read_f64(), y: bor.read_f64() } ],
                        z_array: vec![ bor.read_f64() ],
                        m_array: vec![ bor.read_f64() ],
                        ..Default::default()
                    };
                    self.records.push(sfg);
                }
            },

            ShapeType::PolyLineZ | ShapeType::PolygonZ => {
                while bor.pos < file_size {
                    bor.pos += 8;
                    let mut sfg = ShapefileGeometry {
                        shape_type: ShapeType::from_int(bor.read_i32()), 
                        x_min: bor.read_f64(),
                        y_min: bor.read_f64(),
                        x_max: bor.read_f64(),
                        y_max: bor.read_f64(),
                        num_parts: bor.read_i32(),
                        num_points: bor.read_i32(), 
                        ..Default::default()
                    };

                    for _ in 0..sfg.num_parts {
                        sfg.parts.push(bor.read_i32());
                    }

                    for _ in 0..sfg.num_points {
                        sfg.points.push(Point2D{ x: bor.read_f64(), y: bor.read_f64() });
                    }  

                    sfg.z_min = bor.read_f64();
                    sfg.z_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.z_array.push(bor.read_f64());
                    }

                    sfg.m_min = bor.read_f64();
                    sfg.m_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.m_array.push(bor.read_f64());
                    }  

                    self.records.push(sfg);
                }
            },

            ShapeType::MultiPointZ => {
                while bor.pos < file_size {
                    bor.pos += 8;
                    let mut sfg = ShapefileGeometry {
                        shape_type: ShapeType::from_int(bor.read_i32()), 
                        x_min: bor.read_f64(),
                        y_min: bor.read_f64(),
                        x_max: bor.read_f64(),
                        y_max: bor.read_f64(),
                        num_points: bor.read_i32(), 
                        ..Default::default()
                    };

                    for _ in 0..sfg.num_points {
                        sfg.points.push(Point2D{ x: bor.read_f64(), y: bor.read_f64() });
                    }  

                    sfg.z_min = bor.read_f64();
                    sfg.z_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.z_array.push(bor.read_f64());
                    }

                    sfg.m_min = bor.read_f64();
                    sfg.m_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.m_array.push(bor.read_f64());
                    }

                    self.records.push(sfg);
                }
            },

            ShapeType::PointM => {
                while bor.pos < file_size {
                    bor.pos += 12; // Don't need to read the header and shapeType. It's not going to change.
                    let sfg = ShapefileGeometry {
                        shape_type: ShapeType::Point, 
                        num_points: 1i32, 
                        points: vec![ Point2D{ x: bor.read_f64(), y: bor.read_f64() } ],
                        m_array: vec![ bor.read_f64() ],
                        ..Default::default()
                    };
                    self.records.push(sfg);
                }
            },

            ShapeType::PolyLineM | ShapeType::PolygonM => {
                while bor.pos < file_size {
                    bor.pos += 8;
                    let mut sfg = ShapefileGeometry {
                        shape_type: ShapeType::from_int(bor.read_i32()), 
                        x_min: bor.read_f64(),
                        y_min: bor.read_f64(),
                        x_max: bor.read_f64(),
                        y_max: bor.read_f64(),
                        num_parts: bor.read_i32(),
                        num_points: bor.read_i32(), 
                        ..Default::default()
                    };

                    for _ in 0..sfg.num_parts {
                        sfg.parts.push(bor.read_i32());
                    }

                    for _ in 0..sfg.num_points {
                        sfg.points.push(Point2D{ x: bor.read_f64(), y: bor.read_f64() });
                    }  

                    sfg.m_min = bor.read_f64();
                    sfg.m_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.m_array.push(bor.read_f64());
                    }  

                    self.records.push(sfg);
                }
            },

            ShapeType::MultiPointM => {
                while bor.pos < file_size {
                    bor.pos += 8;
                    
                    let mut sfg = ShapefileGeometry {
                        shape_type: ShapeType::from_int(bor.read_i32()), 
                        x_min: bor.read_f64(),
                        y_min: bor.read_f64(),
                        x_max: bor.read_f64(),
                        y_max: bor.read_f64(),
                        num_points: bor.read_i32(), 
                        ..Default::default()
                    };

                    for _ in 0..sfg.num_points {
                        sfg.points.push(Point2D{ x: bor.read_f64(), y: bor.read_f64() });
                    }  

                    sfg.m_min = bor.read_f64();
                    sfg.m_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.m_array.push(bor.read_f64());
                    }

                    self.records.push(sfg);
                }
            },

            _ => {
                return Err(Error::new(ErrorKind::InvalidInput, "Unrecognized ShapeType."));
            },
        }

        self.num_records = self.records.len();


        //////////////////////////////
        // Read the projection file //
        //////////////////////////////
        let prj_file = self.file_name.replace(".shp", ".prj");
         match File::open(prj_file) {
            Ok(f) => {
                let f = BufReader::new(f);
                for line in f.lines() {
                    let line_unwrapped = line.unwrap();
                    self.projection.push_str(&line_unwrapped);
                }
            },
            Err(_) => { println!("Warning: Projection file not located.") },
        }


        ///////////////////////////////
        // Read the attributes table //
        ///////////////////////////////

        // read the header
        let dbf_file = self.file_name.replace(".shp", ".dbf");
        let mut f = File::open(dbf_file.clone()).unwrap();
        let metadata = fs::metadata(dbf_file.clone()).unwrap();
        let file_size: usize = metadata.len() as usize;
        let mut buffer = vec![0; file_size];

        // read the file's bytes into a buffer
        f.read(&mut buffer).unwrap();

        let mut bor = ByteOrderReader::new(buffer, Endianness::LittleEndian);

        self.attributes.header.version = bor.read_u8();
        self.attributes.header.year = 1900u32 + bor.read_u8() as u32;
        self.attributes.header.month = bor.read_u8();
        self.attributes.header.day = bor.read_u8();
        self.attributes.header.num_records = bor.read_u32();
        self.attributes.header.bytes_in_header = bor.read_u16();
        self.attributes.header.bytes_in_record = bor.read_u16();
        // reserved bytes
        bor.pos += 2;
        self.attributes.header.incomplete_tansaction = bor.read_u8();
        self.attributes.header.encryption_flag = bor.read_u8();
        // skip free record thread for LAN only
        bor.pos += 4;
        // reserved for multi-user dBASE in dBASE III+
        bor.pos += 8;
        self.attributes.header.mdx_flag = bor.read_u8();
        self.attributes.header.language_driver_id = bor.read_u8();
        // reserved bytes
        bor.pos += 2;

        // read the field data
        self.attributes.fields = vec![];
        let mut flag = true;
        while flag {
            let name = bor.read_utf8(11).replace(char::from(0), ""); //(cast[string](buf[bor.pos..bor.pos+10])).strip.replaceNullCharacters
            let field_type = char::from(bor.read_u8());
            bor.pos += 4;
            let field_length = bor.read_u8();
            let decimal_count = bor.read_u8();
            // Skip reserved bytes multi-user dBASE.
            bor.pos += 2;
            let work_area_id = bor.read_u8();
            // Skip reserved bytes multi-user dBASE.
            bor.pos += 2;
            let set_field_flag = bor.read_u8();
            // Skip reserved bytes.
            bor.pos += 7;
            let index_field_flag = bor.read_u8();
            
            let field_data = AttributeField::new(
                &name, 
                field_type, 
                field_length, 
                decimal_count,
                work_area_id,
                set_field_flag,
                index_field_flag
            );

            self.attributes.fields.push(field_data);

            // Checks for end of field descriptor array (0x0d). Valid .dbf files
            // will have this flag.
            if bor.peek_u8() == 0x0d {
                flag = false;
                // break;
            }
        }

        self.attributes.header.num_fields = self.attributes.fields.len() as u32;

        bor.pos += 1;

        let mut d: bool;
        let mut str_rep: String;
        for _i in 0..self.attributes.header.num_records {
            d = bor.read_u8() as u32 == 0x2A;
            let mut r: Vec<FieldData> = vec![];
            for j in 0..self.attributes.header.num_fields {
                str_rep = bor.read_utf8(self.attributes.fields[j as usize].field_length as usize).replace(char::from(0), "").replace("*", "").trim().to_string();
                if str_rep.replace(" ", "").replace("?", "").is_empty() {
                    r.push(FieldData::Null);
                } else {
                    match self.attributes.fields[j as usize].field_type {
                        'N' | 'F' | 'I' | 'O' => {
                            if self.attributes.fields[j as usize].decimal_count == 0 {
                                r.push(FieldData::Int64(str_rep.parse::<i64>().unwrap()));
                            } else {
                                r.push(FieldData::Real(str_rep.parse::<f64>().unwrap()));
                            }
                        },
                        'D' => {
                            r.push(FieldData::Date(DateData{
                                year: str_rep[0..4].parse::<u16>().unwrap(), 
                                month: str_rep[4..6].parse::<u8>().unwrap(), 
                                day: str_rep[6..8].parse::<u8>().unwrap()
                            }));
                        },
                        'L' => {
                            if str_rep.to_lowercase().contains("t") {
                                r.push(FieldData::Bool(true));
                            } else {
                                r.push(FieldData::Bool(false));
                            }
                        }
                        _ => {
                            // treat it like a string
                            r.push(FieldData::Text(str_rep.clone()));
                        }
                    }
                }
            }
            self.attributes.add_record(d, r);
        }

        Ok(())
    }
}
