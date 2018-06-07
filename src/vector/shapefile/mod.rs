/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: 12/04/2018
License: MIT

Notes: The logic behind working with the ESRI Shapefile format.
*/
extern crate time;

pub mod attributes;
pub mod geometry;
pub use self::attributes::{AttributeField, AttributeHeader, DateData, FieldData,
                           ShapefileAttributes};
pub use self::geometry::{ShapeType, ShapefileGeometry};
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use io_utils::{ByteOrderReader, Endianness};
use std::f64;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Error, ErrorKind};
use std::str;
use vector::Point2D;

// 100 bytes in size
#[derive(Default, Clone)]
pub struct ShapefileHeader {
    file_code: i32,            // BigEndian; value is 9994
    unused1: i32,              // BigEndian
    unused2: i32,              // BigEndian
    unused3: i32,              // BigEndian
    unused4: i32,              // BigEndian
    unused5: i32,              // BigEndian
    pub file_length: i32,      // BigEndian
    pub version: i32,          // LittleEndian
    pub shape_type: ShapeType, // LittleEndian
    pub x_min: f64,            // LittleEndian
    pub y_min: f64,            // LittleEndian
    pub x_max: f64,            // LittleEndian
    pub y_max: f64,            // LittleEndian
    pub z_min: f64,            // LittleEndian; set to 0f64 if shapeType not z or measured
    pub z_max: f64,            // LittleEndian; set to 0f64 if shapeType not z or measured
    pub m_min: f64,            // LittleEndian; set to 0f64 if shapeType not z or measured
    pub m_max: f64,            // LittleEndian; set to 0f64 if shapeType not z or measured
}

impl fmt::Display for ShapefileHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!(
            "file_code: {}
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
            self.m_max
        );
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

    pub fn initialize_using_file<'a>(
        file_name: &'a str,
        other: &'a Shapefile,
        shape_type: ShapeType,
        copy_fields: bool,
    ) -> Result<Shapefile, Error> {
        let new_file_name = if file_name.contains(".") {
            file_name.to_string()
        } else {
            // likely no extension provided; default to .shp
            format!("{}.shp", file_name)
        };

        let mut sf = Shapefile {
            file_name: new_file_name,
            file_mode: "w".to_string(),
            projection: other.projection.clone(),
            ..Default::default()
        };
        sf.header.shape_type = shape_type;
        if copy_fields {
            sf.attributes.fields = other.attributes.fields.clone();
            sf.attributes.header.num_fields = sf.attributes.fields.len() as u32;
        }
        Ok(sf)
    }

    /// Returns the ShapefileGeometry for a specified index, starting at zero.
    pub fn get_record<'a>(&'a self, index: usize) -> &'a ShapefileGeometry {
        if index >= self.records.len() {
            panic!("Record index out of bounds");
        }
        &self.records[index]
    }

    /// Adds a new ShapefileGeometry.
    pub fn add_record(&mut self, geometry: ShapefileGeometry) {
        if self.file_mode == "r" {
            panic!("The file was opened in read-only mode.");
        }
        if geometry.shape_type == self.header.shape_type {
            self.records.push(geometry);
            self.num_records += 1;
        } else {
            panic!("Attempt to add a ShapefileGeometry record of the wrong ShapeType.");
        }
    }

    /// Adds a new Point record.
    pub fn add_point_record(&mut self, x: f64, y: f64) {
        if self.file_mode == "r" {
            panic!("The file was opened in read-only mode.");
        }
        if self.header.shape_type == ShapeType::Point {
            let mut sfg = ShapefileGeometry {
                shape_type: ShapeType::Point,
                ..Default::default()
            };
            sfg.add_point(Point2D { x: x, y: y });
            self.records.push(sfg);
            self.num_records += 1;
        } else {
            panic!("Attempt to add a ShapefileGeometry record of the wrong ShapeType.");
        }
    }

    pub fn read(&mut self) -> Result<(), Error> {
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
                        points: vec![Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        }],
                        ..Default::default()
                    };
                    self.records.push(sfg);
                }
            }

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
                        sfg.points.push(Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        });
                    }

                    self.records.push(sfg);
                }
            }

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
                        sfg.points.push(Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        });
                    }

                    self.records.push(sfg);
                }
            }

            ShapeType::PointZ => {
                while bor.pos < file_size {
                    bor.pos += 12; // Don't need to read the header and shapeType. It's not going to change.
                    let sfg = ShapefileGeometry {
                        shape_type: ShapeType::Point,
                        num_points: 1i32,
                        points: vec![Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        }],
                        z_array: vec![bor.read_f64()],
                        m_array: vec![bor.read_f64()],
                        ..Default::default()
                    };
                    self.records.push(sfg);
                }
            }

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
                        sfg.points.push(Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        });
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
            }

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
                        sfg.points.push(Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        });
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
            }

            ShapeType::PointM => {
                while bor.pos < file_size {
                    bor.pos += 12; // Don't need to read the header and shapeType. It's not going to change.
                    let sfg = ShapefileGeometry {
                        shape_type: ShapeType::Point,
                        num_points: 1i32,
                        points: vec![Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        }],
                        m_array: vec![bor.read_f64()],
                        ..Default::default()
                    };
                    self.records.push(sfg);
                }
            }

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
                        sfg.points.push(Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        });
                    }

                    sfg.m_min = bor.read_f64();
                    sfg.m_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.m_array.push(bor.read_f64());
                    }

                    self.records.push(sfg);
                }
            }

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
                        sfg.points.push(Point2D {
                            x: bor.read_f64(),
                            y: bor.read_f64(),
                        });
                    }

                    sfg.m_min = bor.read_f64();
                    sfg.m_max = bor.read_f64();
                    for _ in 0..sfg.num_points {
                        sfg.m_array.push(bor.read_f64());
                    }

                    self.records.push(sfg);
                }
            }

            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Unrecognized ShapeType.",
                ));
            }
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
                    self.projection.push_str(&format!("{}\n", line_unwrapped));
                }
            }
            Err(_) => println!("Warning: Projection file not located."),
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
                index_field_flag,
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
        for _ in 0..self.attributes.header.num_records {
            d = bor.read_u8() as u32 == 0x2A;
            let mut r: Vec<FieldData> = vec![];
            for j in 0..self.attributes.header.num_fields {
                str_rep = bor.read_utf8(self.attributes.fields[j as usize].field_length as usize)
                    .replace(char::from(0), "")
                    .replace("*", "")
                    .trim()
                    .to_string();
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
                        }
                        'D' => {
                            r.push(FieldData::Date(DateData {
                                year: str_rep[0..4].parse::<u16>().unwrap(),
                                month: str_rep[4..6].parse::<u8>().unwrap(),
                                day: str_rep[6..8].parse::<u8>().unwrap(),
                            }));
                        }
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

    pub fn write(&mut self) -> Result<(), Error> {
        if self.file_mode == "r" {
            return Err(Error::new(
                ErrorKind::Other,
                "The file was opened in read-only mode.",
            ));
        }

        self.num_records = self.records.len(); // make sure they are the same.
        if self.num_records == 0 {
            return Err(Error::new(
                ErrorKind::Other,
                "The file does not currently contain any record data.",
            ));
        }

        ///////////////////////////////////////////////
        // First write the geometry data (.shp file) //
        ///////////////////////////////////////////////

        // write the header
        let f = File::create(&self.file_name)?;
        let mut writer = BufWriter::new(f);

        // magic number
        writer.write_i32::<BigEndian>(9994i32)?;

        // unused header bytes
        for _ in 0..5 {
            writer.write_i32::<BigEndian>(0i32)?;
        }

        // file size
        let mut size = 100i32; // initialized to the size of the file header
        for i in 0..self.num_records {
            size += 8 + self.records[i].get_length();
        }
        let file_length = size / 2i32; // in 16-bit words
        writer.write_i32::<BigEndian>(file_length)?;

        // version
        writer.write_i32::<LittleEndian>(1000i32)?;

        // shape type
        writer.write_i32::<LittleEndian>(self.header.shape_type.to_int())?;

        // extent
        self.calculate_extent();
        writer.write_f64::<LittleEndian>(self.header.x_min)?;
        writer.write_f64::<LittleEndian>(self.header.y_min)?;
        writer.write_f64::<LittleEndian>(self.header.x_max)?;
        writer.write_f64::<LittleEndian>(self.header.y_max)?;
        writer.write_f64::<LittleEndian>(self.header.z_min)?;
        writer.write_f64::<LittleEndian>(self.header.z_max)?;
        writer.write_f64::<LittleEndian>(self.header.m_min)?;
        writer.write_f64::<LittleEndian>(self.header.m_max)?;

        // Write the geometries
        match self.header.shape_type {
            ShapeType::Null => {
                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(0i32)?; // Shape type
                }
            }
            ShapeType::Point => {
                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(1i32)?; // Shape type
                    writer.write_f64::<LittleEndian>(self.records[i].points[0].x)?;
                    writer.write_f64::<LittleEndian>(self.records[i].points[0].y)?;
                }
            }

            ShapeType::PolyLine | ShapeType::Polygon => {
                let st = if self.header.shape_type == ShapeType::PolyLine {
                    3i32
                } else {
                    5i32
                };

                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(st)?; // Shape type

                    // extent
                    writer.write_f64::<LittleEndian>(self.records[i].x_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].x_max)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_max)?;

                    writer.write_i32::<LittleEndian>(self.records[i].num_parts)?; // Num parts
                    writer.write_i32::<LittleEndian>(self.records[i].num_points)?; // Num points

                    // parts
                    for part in &self.records[i].parts {
                        writer.write_i32::<LittleEndian>(*part)?;
                    }

                    // points
                    for pt in &self.records[i].points {
                        writer.write_f64::<LittleEndian>(pt.x)?;
                        writer.write_f64::<LittleEndian>(pt.y)?;
                    }
                }
            }

            ShapeType::MultiPoint => {
                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(8i32)?; // Shape type

                    // extent
                    writer.write_f64::<LittleEndian>(self.records[i].x_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].x_max)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_max)?;

                    writer.write_i32::<LittleEndian>(self.records[i].num_points)?; // Num points

                    // points
                    for pt in &self.records[i].points {
                        writer.write_f64::<LittleEndian>(pt.x)?;
                        writer.write_f64::<LittleEndian>(pt.y)?;
                    }
                }
            }

            ShapeType::PointZ => {
                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(11i32)?; // Shape type
                    writer.write_f64::<LittleEndian>(self.records[i].points[0].x)?;
                    writer.write_f64::<LittleEndian>(self.records[i].points[0].y)?;
                    writer.write_f64::<LittleEndian>(self.records[i].z_array[0])?;
                    writer.write_f64::<LittleEndian>(self.records[i].m_array[0])?;
                }
            }

            ShapeType::PolyLineZ | ShapeType::PolygonZ => {
                let st = if self.header.shape_type == ShapeType::PolyLine {
                    13i32
                } else {
                    15i32
                };

                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(st)?; // Shape type

                    // extent
                    writer.write_f64::<LittleEndian>(self.records[i].x_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].x_max)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_max)?;

                    writer.write_i32::<LittleEndian>(self.records[i].num_parts)?; // Num parts
                    writer.write_i32::<LittleEndian>(self.records[i].num_points)?; // Num points

                    // parts
                    for part in &self.records[i].parts {
                        writer.write_i32::<LittleEndian>(*part)?;
                    }

                    // points
                    for pt in &self.records[i].points {
                        writer.write_f64::<LittleEndian>(pt.x)?;
                        writer.write_f64::<LittleEndian>(pt.y)?;
                    }

                    // z data
                    writer.write_f64::<LittleEndian>(self.records[i].z_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].z_max)?;
                    for z in &self.records[i].z_array {
                        writer.write_f64::<LittleEndian>(*z)?;
                    }

                    // measure data
                    writer.write_f64::<LittleEndian>(self.records[i].m_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].m_max)?;
                    for m in &self.records[i].m_array {
                        writer.write_f64::<LittleEndian>(*m)?;
                    }
                }
            }

            ShapeType::MultiPointZ => {
                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(18i32)?; // Shape type

                    // extent
                    writer.write_f64::<LittleEndian>(self.records[i].x_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].x_max)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_max)?;

                    writer.write_i32::<LittleEndian>(self.records[i].num_points)?; // Num points

                    // points
                    for pt in &self.records[i].points {
                        writer.write_f64::<LittleEndian>(pt.x)?;
                        writer.write_f64::<LittleEndian>(pt.y)?;
                    }

                    // z data
                    writer.write_f64::<LittleEndian>(self.records[i].z_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].z_max)?;
                    for z in &self.records[i].z_array {
                        writer.write_f64::<LittleEndian>(*z)?;
                    }

                    // measure data
                    writer.write_f64::<LittleEndian>(self.records[i].m_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].m_max)?;
                    for m in &self.records[i].m_array {
                        writer.write_f64::<LittleEndian>(*m)?;
                    }
                }
            }

            ShapeType::PointM => {
                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(21i32)?; // Shape type
                    writer.write_f64::<LittleEndian>(self.records[i].points[0].x)?;
                    writer.write_f64::<LittleEndian>(self.records[i].points[0].y)?;
                    writer.write_f64::<LittleEndian>(self.records[i].m_array[0])?;
                }
            }

            ShapeType::PolyLineM | ShapeType::PolygonM => {
                let st = if self.header.shape_type == ShapeType::PolyLine {
                    23i32
                } else {
                    25i32
                };

                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(st)?; // Shape type

                    // extent
                    writer.write_f64::<LittleEndian>(self.records[i].x_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].x_max)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_max)?;

                    writer.write_i32::<LittleEndian>(self.records[i].num_parts)?; // Num parts
                    writer.write_i32::<LittleEndian>(self.records[i].num_points)?; // Num points

                    // parts
                    for part in &self.records[i].parts {
                        writer.write_i32::<LittleEndian>(*part)?;
                    }

                    // points
                    for pt in &self.records[i].points {
                        writer.write_f64::<LittleEndian>(pt.x)?;
                        writer.write_f64::<LittleEndian>(pt.y)?;
                    }

                    // measure data
                    writer.write_f64::<LittleEndian>(self.records[i].m_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].m_max)?;
                    for m in &self.records[i].m_array {
                        writer.write_f64::<LittleEndian>(*m)?;
                    }
                }
            }

            ShapeType::MultiPointM => {
                for i in 0..self.num_records {
                    writer.write_i32::<BigEndian>(i as i32 + 1i32)?; // Record number
                    writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
                    writer.write_i32::<LittleEndian>(28i32)?; // Shape type

                    // extent
                    writer.write_f64::<LittleEndian>(self.records[i].x_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].x_max)?;
                    writer.write_f64::<LittleEndian>(self.records[i].y_max)?;

                    writer.write_i32::<LittleEndian>(self.records[i].num_points)?; // Num points

                    // points
                    for pt in &self.records[i].points {
                        writer.write_f64::<LittleEndian>(pt.x)?;
                        writer.write_f64::<LittleEndian>(pt.y)?;
                    }

                    // measure data
                    writer.write_f64::<LittleEndian>(self.records[i].m_min)?;
                    writer.write_f64::<LittleEndian>(self.records[i].m_max)?;
                    for m in &self.records[i].m_array {
                        writer.write_f64::<LittleEndian>(*m)?;
                    }
                }
            }
        }

        /////////////////////////////////
        // Write the index file (.shx) //
        /////////////////////////////////

        // write the header
        let index_file = self.file_name.replace(".shp", ".shx");
        let f = File::create(&index_file)?;
        let mut writer = BufWriter::new(f);

        // magic number
        writer.write_i32::<BigEndian>(9994i32)?;

        // unused header bytes
        for _ in 0..5 {
            writer.write_i32::<BigEndian>(0i32)?;
        }

        let file_length = (100 + 8 * self.num_records) as i32 / 2i32; // in 16-bit words
        writer.write_i32::<BigEndian>(file_length)?;

        // version
        writer.write_i32::<LittleEndian>(1000i32)?;

        // shape type
        writer.write_i32::<LittleEndian>(self.header.shape_type.to_int())?;

        // extent
        self.calculate_extent();
        writer.write_f64::<LittleEndian>(self.header.x_min)?;
        writer.write_f64::<LittleEndian>(self.header.y_min)?;
        writer.write_f64::<LittleEndian>(self.header.x_max)?;
        writer.write_f64::<LittleEndian>(self.header.y_max)?;
        writer.write_f64::<LittleEndian>(self.header.z_min)?;
        writer.write_f64::<LittleEndian>(self.header.z_max)?;
        writer.write_f64::<LittleEndian>(self.header.m_min)?;
        writer.write_f64::<LittleEndian>(self.header.m_max)?;

        let mut pos = 100i32;

        for i in 0..self.num_records {
            writer.write_i32::<BigEndian>(pos / 2)?; // Record number
            writer.write_i32::<BigEndian>(self.records[i].get_length() / 2)?; // Content length in 16-bit words
            pos += 8 + self.records[i].get_length();
        }

        ///////////////////////////////
        // Write the projection file //
        ///////////////////////////////

        if !self.projection.is_empty() {
            let prj_file = self.file_name.replace(".shp", ".prj");
            let f = File::create(&prj_file)?;
            let mut writer = BufWriter::new(f);
            writer.write_all(self.projection.as_bytes())?;
        }

        ///////////////////////////////
        // Write the attributes file //
        ///////////////////////////////

        // let dbf_file = self.file_name.replace(".shp", ".dbf");
        // let f = File::create(&dbf_file)?;
        // let mut writer = BufWriter::new(f);

        // writer.write_u8(3u8)?;

        // // write the date
        // let now = time::now();
        // writer.write_u8(now.tm_year as u8)?;
        // writer.write_u8(now.tm_mon as u8 + 1u8)?;
        // writer.write_u8(now.tm_mday as u8)?;

        // writer.write_u32::<LittleEndian>(self.attributes.header.num_records)?; // number of records
        // let header_size = 68u32 + self.attributes.header.num_fields * 32 + 1; // maybe should be 64 instead...check java code.
        // writer.write_u32::<LittleEndian>(header_size)?; // header size

        // let bytes_in_record = 0u32;
        // writer.write_u32::<LittleEndian>(bytes_in_record)?; // bytes in record

        // // reserved or unused bytes
        // for _ in 0..20 {
        //     writer.write_u8(0u8)?;
        // }

        // // Field descriptor array
        // for field in &self.attributes.fields {
        //     let mut s = field.name.clone();
        //     if s.len() > 10 {
        //         s = field.name[0..10].to_string();
        //     }
        //     for _ in s.len()..11 {
        //         s.push(char::from(0));
        //     }
        //     writer.write_all(s.as_bytes())?;
        //     writer.write_u8(field.field_type as u8)?;

        //     for _ in 0..4 {
        //         writer.write_u8(0u8)?;
        //     }

        //     writer.write_u8(field.field_length)?;
        //     writer.write_u8(field.decimal_count)?;

        //     for _ in 0..14 {
        //         writer.write_u8(0u8)?;
        //     }
        // }

        // writer.write_u8(0x0D)?; // terminator byte

        // // write records
        // for i in 0..self.attributes.header.num_records as usize {
        //     writer.write_u8(0x20)?;
        //     let rec = self.attributes.get_record(i);
        //     for j in 0..self.attributes.header.num_fields {
        //         let fl = self.attributes.fields[j as usize].field_length;
        //         match &rec[j as usize] {
        //             FieldData::Null => {
        //                 writer.write_all("?".as_bytes())?;
        //             }
        //             FieldData::Int(v) => {
        //                 writer.write_all(&format!("{}", v).as_bytes())?;
        //             }
        //             FieldData::Int64(v) => {
        //                 writer.write_all(&format!("{}", v).as_bytes())?;
        //             }
        //             _ => {} // do nothing
        //         }
        //     }
        // }

        Ok(())
    }

    fn calculate_extent(&mut self) {
        match self.header.shape_type {
            ShapeType::Null => {
                self.header.x_min = 0f64;
                self.header.x_max = 0f64;
                self.header.y_min = 0f64;
                self.header.y_max = 0f64;
                self.header.m_min = 0f64;
                self.header.m_max = 0f64;
                self.header.z_min = 0f64;
                self.header.z_max = 0f64;
            }
            ShapeType::Point | ShapeType::PolyLine | ShapeType::Polygon | ShapeType::MultiPoint => {
                self.header.x_min = f64::INFINITY;
                self.header.x_max = f64::NEG_INFINITY;
                self.header.y_min = f64::INFINITY;
                self.header.y_max = f64::NEG_INFINITY;
                self.header.m_min = 0f64;
                self.header.m_max = 0f64;
                self.header.z_min = 0f64;
                self.header.z_max = 0f64;

                for sg in &self.records {
                    if sg.x_min < self.header.x_min {
                        self.header.x_min = sg.x_min;
                    }
                    if sg.y_min < self.header.y_min {
                        self.header.y_min = sg.y_min;
                    }

                    if sg.x_max > self.header.x_max {
                        self.header.x_max = sg.x_max;
                    }
                    if sg.y_max > self.header.y_max {
                        self.header.y_max = sg.y_max;
                    }
                }
            }
            ShapeType::PointM
            | ShapeType::PolyLineM
            | ShapeType::PolygonM
            | ShapeType::MultiPointM => {
                self.header.x_min = f64::INFINITY;
                self.header.x_max = f64::NEG_INFINITY;
                self.header.y_min = f64::INFINITY;
                self.header.y_max = f64::NEG_INFINITY;
                self.header.m_min = f64::INFINITY;
                self.header.m_max = f64::NEG_INFINITY;
                self.header.z_min = 0f64;
                self.header.z_max = 0f64;

                for sg in &self.records {
                    if sg.x_min < self.header.x_min {
                        self.header.x_min = sg.x_min;
                    }
                    if sg.y_min < self.header.y_min {
                        self.header.y_min = sg.y_min;
                    }
                    if sg.m_min < self.header.m_min {
                        self.header.m_min = sg.m_min;
                    }

                    if sg.x_max > self.header.x_max {
                        self.header.x_max = sg.x_max;
                    }
                    if sg.y_max > self.header.y_max {
                        self.header.y_max = sg.y_max;
                    }
                    if sg.m_max > self.header.m_max {
                        self.header.m_max = sg.m_max;
                    }
                }
            }
            ShapeType::PointZ
            | ShapeType::PolyLineZ
            | ShapeType::PolygonZ
            | ShapeType::MultiPointZ => {
                self.header.x_min = f64::INFINITY;
                self.header.x_max = f64::NEG_INFINITY;
                self.header.y_min = f64::INFINITY;
                self.header.y_max = f64::NEG_INFINITY;
                self.header.m_min = f64::INFINITY;
                self.header.m_max = f64::NEG_INFINITY;
                self.header.z_min = f64::INFINITY;
                self.header.z_max = f64::NEG_INFINITY;

                for sg in &self.records {
                    if sg.x_min < self.header.x_min {
                        self.header.x_min = sg.x_min;
                    }
                    if sg.y_min < self.header.y_min {
                        self.header.y_min = sg.y_min;
                    }
                    if sg.m_min < self.header.m_min {
                        self.header.m_min = sg.m_min;
                    }
                    if sg.z_min < self.header.z_min {
                        self.header.z_min = sg.z_min;
                    }

                    if sg.x_max > self.header.x_max {
                        self.header.x_max = sg.x_max;
                    }
                    if sg.y_max > self.header.y_max {
                        self.header.y_max = sg.y_max;
                    }
                    if sg.m_max > self.header.m_max {
                        self.header.m_max = sg.m_max;
                    }
                    if sg.z_max > self.header.z_max {
                        self.header.z_max = sg.z_max;
                    }
                }
            }
        }
    }
}
