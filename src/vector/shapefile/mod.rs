/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: July 17, 2017
License: MIT
*/
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
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

        Ok(())
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

impl ShapefileGeometry {
    pub fn is_hole(&self, part_num: i32) -> bool {
        // see if it's a polygon
        if self.shape_type.base_shape_type() != ShapeType::Polygon {
            // it's not a polygon
            return false;
        }
        // it is a polygon

        if part_num < 0 || part_num > self.num_parts - 1 {
            // it's not a real part
            return false;
        }
        
        // Note: holes are polygons that have verticies in counter-clockwise order

        // This approach is based on the method described by Paul Bourke, March 1998
        // http://paulbourke.net/geometry/clockwise/index.html

        let (mut x0, mut y0, mut x1, mut y1, mut x2, mut y2): (f64, f64, f64, f64, f64, f64);
        let mut n1: usize; 
        let mut n2: usize;
        let mut n3: usize;

        let st_point = self.parts[part_num as usize] as usize;

        let end_point = if part_num < self.num_parts - 2 {
            // remember, the last point in each part is the same as the first...it's not a legitemate point.
            (self.parts[part_num as usize] - 2i32) as usize
        } else {
            (self.num_points - 2i32) as usize
        };

        let num_points_in_part = end_point - st_point + 1;

        if num_points_in_part < 3 {
            return false;
        } // something's wrong!

        // first see if it is a convex or concave polygon
        // calculate the cross product for each adjacent edge.

        let mut crossproducts = vec![0f64; num_points_in_part];
        for j in 0..num_points_in_part {
            n2 = st_point + j;
            if j == 0 {
                n1 = st_point + num_points_in_part - 1;
                n3 = st_point + j + 1;
            } else if j == num_points_in_part - 1 {
                n1 = st_point + j - 1;
                n3 = st_point;
            } else {
                n1 = st_point + j - 1;
                n3 = st_point + j + 1;
            }
            x0 = self.points[n1].x;
            y0 = self.points[n1].y;
            x1 = self.points[n2].x;
            y1 = self.points[n2].y;
            x2 = self.points[n3].x;
            y2 = self.points[n3].y;
            crossproducts[j] = (x1 - x0) * (y2 - y1) - (y1 - y0) * (x2 - x1);
        }

        let test_sign = crossproducts[0] >= 0f64;
        let mut is_convex = true;
        for j in 1..num_points_in_part {
            if crossproducts[j] >= 0f64 && !test_sign {
                is_convex = false;
                break;
            } else if crossproducts[j] < 0f64 && test_sign {
                is_convex = false;
                break;
            }
        }

        // now see if it is clockwise or counter-clockwise
        if is_convex {
            if test_sign { // positive means counter-clockwise
                return true;
            } else {
                return false;
            }
        } else {
            // calculate the polygon area. If it is positive is is in clockwise order, else counter-clockwise.
            let mut area = 0f64;
            for j in 0..num_points_in_part {
                n1 = st_point + j;
                if j < num_points_in_part - 1 {
                    n2 = st_point + j + 1;
                } else {
                    n2 = st_point;
                }
                x1 = self.points[n1].x;
                y1 = self.points[n1].y;
                x2 = self.points[n2].x;
                y2 = self.points[n2].y;

                area += (x1 * y2) - (x2 * y1);
            }
            area /= 2.0;

            if area < 0f64 { // a positive area indicates counter-clockwise order
                return false;
            } else {
                return true;
            }
        }
    }
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

#[repr(u16)]
#[derive(Clone, PartialEq)]
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

    pub fn base_shape_type(&self) -> ShapeType {
        match self {
            &ShapeType::Null => ShapeType::Null,
            &ShapeType::Point | &ShapeType::PointZ | &ShapeType::PointM => ShapeType::Point,
            &ShapeType::PolyLine | &ShapeType::PolyLineZ | &ShapeType::PolyLineM => ShapeType::PolyLine,
            &ShapeType::Polygon | &ShapeType::PolygonZ | &ShapeType::PolygonM => ShapeType::Polygon,
            &ShapeType::MultiPoint | &ShapeType::MultiPointZ | &ShapeType::MultiPointM => ShapeType::MultiPoint,
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