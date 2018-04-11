/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: 10/04/2018
License: MIT

Notes: The logic behind working with the ESRI Shapefile format.
*/
pub mod attributes;
pub mod geometry;

pub use self::attributes::{AttributeField, AttributeHeader, ShapefileAttributes};
pub use self::geometry::{ShapefileGeometry, ShapeType};
use std::io::prelude::*;
use std::io::{BufReader, Error, ErrorKind};
use std::fs;
use std::fs::File;
use std::fmt;
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
        let f = File::open(prj_file)?;
        let f = BufReader::new(f);

        for line in f.lines() {
            let line_unwrapped = line.unwrap();
            self.projection.push_str(&line_unwrapped);
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

        // self.attributes.fields = vec![];
        // let mut flag = true;
        // while flag {
        //     // let mut field_data = AttributeField{..Default::default()};
        //     // fieldData.name = (cast[string](buf[bor.pos..bor.pos+10])).strip.replaceNullCharacters

        //     bor.pos += 11;
        //     // fieldData.fieldType = bor.readUint8.char;
        // //     bor.pos += 4;
        // //     fieldData.fieldLength = bor.readUint8
        // //     fieldData.decimalCount = bor.readUint8

        // //     // Skip reserved bytes multi-user dBASE.
        // //     bor.pos += 2;
        // //     fieldData.workAreaId = bor.readUint8
        // //     // Skip reserved bytes multi-user dBASE.
        // //     bor.pos += 2;
        // //     fieldData.setFieldFlag = bor.readUint8
        // //     // Skip reserved bytes.
        // //     bor.pos += 7;
        // //     fieldData.indexFieldFlag = bor.readUint8
        // //     attr.fields.add(fieldData)

        //     // Checks for end of field descriptor array (0x0d). Valid .dbf files
        //     // will have this flag.
        //     if buf[bor.pos] == 0x0d {
        //         flag = false;
        //         break;
        //     }
        // }

        // attr.header.numberOfFields = len(attr.fields).uint32

        // bor.pos += 1

        // attr.data = @[]
        // attr.deleted = @[] #newSeq[false](attr.header.numberOfRecords)

        // for i in 0..attr.header.numberOfRecords-1:
        //     # var record = {};
        //     # record["recordDeleted"] = String.fromCharCode(dv.getUint8(idx));
        //     let deleted = (bor.readUint8).uint32 == 0x2A
        //     attr.deleted.add(deleted)
        //     # discard bor.readUint8
        //     var r = newSeq[string]()
        //     for j in 0..attr.header.numberOfFields-1:
        //     var charString = newSeq[char]()
        //     for h in 0'u64..(attr.fields[j.int].fieldLength - 1):
        //         charString.add(bor.readUint8.char)

        //     r.add((cast[string](charString)).strip)
        //     # # record[o.fields[j].name] = charString.join('').trim();
        //     # echo val
        //     attr.data.add(r)


        // self.attributes = attr

        Ok(())
    }
}
