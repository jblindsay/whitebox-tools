/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: July 17, 2017
License: MIT
*/
use std::io::prelude::*;
use std::io::Error;
use std::fs;
use std::fs::File;
use std::fmt;
use io_utils::{ByteOrderReader, Endianness};

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
    pub records: Vec<ShapefileRecord>,
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
            sf.read()
        } else {
            // write
            // return Ok(r);
        }
        Ok(sf)
        
    }

    pub fn get_record<'a>(&'a self, index: usize) -> &'a ShapefileRecord {
        if index >= self.records.len() { panic!("Record index out of bounds"); }
        &self.records[index]
    }

    fn read(&mut self) {
        // read the header
        let mut f = File::open(self.file_name.clone()).unwrap(); //?;
        let metadata = fs::metadata(self.file_name.clone()).unwrap(); //?;
        let file_size: usize = metadata.len() as usize;
        // let header_size = 100usize;
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
        match self.header.shape_type {
            ShapeType::Point => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    bor.pos += 4; // Don't need to read the shapeType. It's not going to change.
                    r.geometry = ShapefileGeometry {
                        shape_type: ShapeType::Point, 
                        num_points: 1i32, 
                        points: vec![ Point2D{ x: bor.read_f64(), y: bor.read_f64() } ],
                        ..Default::default()
                    };
                    self.records.push(r);
                }
            },

            ShapeType::PolyLine | ShapeType::Polygon => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    
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

                    r.geometry = sfg;
                    self.records.push(r);
                }
            },

            ShapeType::MultiPoint => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    
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

                    r.geometry = sfg;
                    self.records.push(r);
                }
            },

            ShapeType::PointZ => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    bor.pos += 4; // Don't need to read the shapeType. It's not going to change.
                    r.geometry = ShapefileGeometry {
                        shape_type: ShapeType::Point, 
                        num_points: 1i32, 
                        points: vec![ Point2D{ x: bor.read_f64(), y: bor.read_f64() } ],
                        z_array: vec![ bor.read_f64() ],
                        m_array: vec![ bor.read_f64() ],
                        ..Default::default()
                    };
                    self.records.push(r);
                }
            },

            ShapeType::PolyLineZ | ShapeType::PolygonZ => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    
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

                    r.geometry = sfg;
                    self.records.push(r);
                }
            },

            ShapeType::MultiPointZ => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    
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

                    r.geometry = sfg;
                    self.records.push(r);
                }
            },

            ShapeType::PointM => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    bor.pos += 4; // Don't need to read the shapeType. It's not going to change.
                    r.geometry = ShapefileGeometry {
                        shape_type: ShapeType::Point, 
                        num_points: 1i32, 
                        points: vec![ Point2D{ x: bor.read_f64(), y: bor.read_f64() } ],
                        m_array: vec![ bor.read_f64() ],
                        ..Default::default()
                    };
                    self.records.push(r);
                }
            },

            ShapeType::PolyLineM | ShapeType::PolygonM => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    
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

                    r.geometry = sfg;
                    self.records.push(r);
                }
            },

            ShapeType::MultiPointM => {
                while bor.pos < file_size {
                    bor.byte_order = Endianness::BigEndian;
                    let mut r = ShapefileRecord {
                        record_number: bor.read_i32(), 
                        content_length: bor.read_i32(),
                        ..Default::default()
                    };
                    bor.byte_order = Endianness::LittleEndian;
                    
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

                    r.geometry = sfg;
                    self.records.push(r);
                }
            },

            _ => panic!("Unrecognized ShapeType."),
        }

        self.num_records = self.records.len();
    }
}

#[derive(Default, Clone)]
pub struct ShapefileRecord {
  pub record_number: i32,
  pub content_length: i32,
  pub geometry: ShapefileGeometry,
}

impl fmt::Display for ShapefileRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("record_number: {}
content_length: {}
data: {}", 
            self.record_number, 
            self.content_length,
            self.geometry);
        write!(f, "{}", s)
    }
}

#[derive(Clone)]
pub struct ShapefileGeometry {
  pub shape_type: ShapeType,
  pub x_min: f64,
  pub x_max: f64,
  pub y_min: f64,
  pub y_max: f64,
  pub num_parts: i32,
  pub num_points: i32,
  pub parts: Vec<i32>,
  pub points: Vec<Point2D>,
  pub z_min: f64,
  pub z_max: f64,
  pub z_array: Vec<f64>,
  pub m_min: f64,
  pub m_max: f64,
  pub m_array: Vec<f64>,
}

impl Default for ShapefileGeometry {
    fn default() -> ShapefileGeometry { 
        ShapefileGeometry {
            shape_type: ShapeType::Null,
            x_min: 0f64,
            x_max: 0f64,
            y_min: 0f64,
            y_max: 0f64,
            num_parts: 0i32,
            num_points: 0i32,
            parts: vec![],
            points: vec![],
            z_min: 0f64,
            z_max: 0f64,
            z_array: vec![],
            m_min: 0f64,
            m_max: 0f64,
            m_array: vec![]
        }
    }
}

impl fmt::Display for ShapefileGeometry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("shape_type: {}
x_min: {}
x_max: {}
y_min: {}
y_max: {}
num_parts: {}
num_points: {}
parts: {:?}
points: {:?}
z_min: {}
z_max: {}
z_array: {:?}
m_min: {}
m_max: {}
m_array: {:?}", 
            self.shape_type, 
            self.x_min,
            self.x_max,
            self.y_min,
            self.y_max,
            self.num_parts,
            self.num_points,
            self.parts,
            self.points,
            self.z_min,
            self.z_max,
            self.z_array,
            self.m_min,
            self.m_max,
            self.m_array);
        write!(f, "{}", s)
    }
}

#[derive(Default, Clone, Debug)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl fmt::Display for Point2D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("(x: {}, y: {})", self.x, self.y);
        write!(f, "{}", s)
    }
}

#[repr(u16)]
#[derive(Clone)]
pub enum ShapeType { 
    Null = 0,
    Point = 1, 
    PolyLine = 3,
    Polygon = 5,
    MultiPoint = 8,
    PointZ = 11,
    PolyLineZ = 13,
    PolygonZ = 15,
    MultiPointZ = 18,
    PointM = 21,
    PolyLineM = 23,
    PolygonM = 25,
    MultiPointM = 28,
}

impl ShapeType {
    pub fn from_int(value: i32) -> ShapeType {
        match value {
            0 => return ShapeType::Null,
            1 => return ShapeType::Point,
            3 => return ShapeType::PolyLine,
            5 => return ShapeType::Polygon,
            8 => return ShapeType::MultiPoint,
            11 => return ShapeType::PointZ,
            13 => return ShapeType::PolyLineZ,
            15 => return ShapeType::PolygonZ,
            18 => return ShapeType::MultiPointZ,
            21 => return ShapeType::PointM,
            23 => return ShapeType::PolyLineM,
            25 => return ShapeType::PolygonM,
            28 => return ShapeType::MultiPointM,
            _ => panic!("Unrecognized ShapeType")
        }
    }
}

impl Default for ShapeType {
    fn default() -> ShapeType { ShapeType::Null }
}

impl fmt::Display for ShapeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
         let printable = match *self {
            ShapeType::Null => "Null",
            ShapeType::Point => "Point",
            ShapeType::PolyLine => "PolyLine",
            ShapeType::Polygon => "Polygon",
            ShapeType::MultiPoint => "MultiPoint",
            ShapeType::PointZ => "PointZ",
            ShapeType::PolyLineZ => "PolyLineZ",
            ShapeType::PolygonZ => "PolygonZ",
            ShapeType::MultiPointZ => "MultiPointZ",
            ShapeType::PointM => "PointM",
            ShapeType::PolyLineM => "PolyLineM",
            ShapeType::PolygonM => "PolygonM",
            ShapeType::MultiPointM => "MultiPointM",
        };
        write!(f, "{}", printable)
    }
}