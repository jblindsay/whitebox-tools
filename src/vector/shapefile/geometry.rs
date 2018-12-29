/*
This file is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/04/2018
Last Modified: 30/09/2018
License: MIT
*/
use crate::algorithms::point_in_poly;
use crate::structures::{BoundingBox, Point2D};
use std::f64;
use std::fmt;

#[derive(Clone, Debug)]
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
    /// ShapefileGeometry constructor method.
    pub fn new(shape_type: ShapeType) -> ShapefileGeometry {
        ShapefileGeometry {
            shape_type: shape_type,
            ..Default::default()
        }
    }

    /// Adds a single Point2D to the ShapefileGeometry's points array.
    pub fn add_point(&mut self, p: Point2D) {
        self.points.push(p);
        self.num_points += 1;
        if p.x < self.x_min {
            self.x_min = p.x;
        }
        if p.x > self.x_max {
            self.x_max = p.x;
        }
        if p.y < self.y_min {
            self.y_min = p.y;
        }
        if p.y > self.y_max {
            self.y_max = p.y;
        }
    }

    /// Adds a single Point2D and measure to the ShapefileGeometry.
    pub fn add_pointm(&mut self, p: Point2D, m: f64) {
        self.points.push(p);
        self.m_array.push(m);
        self.num_points += 1;
        if p.x < self.x_min {
            self.x_min = p.x;
        }
        if p.x > self.x_max {
            self.x_max = p.x;
        }
        if p.y < self.y_min {
            self.y_min = p.y;
        }
        if p.y > self.y_max {
            self.y_max = p.y;
        }
        if m < self.m_min {
            self.m_min = m;
        }
        if m > self.m_max {
            self.m_max = m;
        }
    }

    /// Adds a single Point2D, measure, and z-value to the ShapefileGeometry.
    pub fn add_pointz(&mut self, p: Point2D, m: f64, z: f64) {
        self.points.push(p);
        self.m_array.push(m);
        self.z_array.push(z);
        self.num_points += 1;
        if p.x < self.x_min {
            self.x_min = p.x;
        }
        if p.x > self.x_max {
            self.x_max = p.x;
        }
        if p.y < self.y_min {
            self.y_min = p.y;
        }
        if p.y > self.y_max {
            self.y_max = p.y;
        }
        if m < self.m_min {
            self.m_min = m;
        }
        if m > self.m_max {
            self.m_max = m;
        }
        if z < self.z_min {
            self.z_min = z;
        }
        if z > self.z_max {
            self.z_max = z;
        }
    }

    /// Adds a part of Point2Ds to the ShapefileGeometry.
    pub fn add_part(&mut self, points: &[Point2D]) {
        self.parts.push(self.points.len() as i32);
        self.num_parts += 1i32;

        for p in points {
            self.points.push(*p);
            if p.x < self.x_min {
                self.x_min = p.x;
            }
            if p.x > self.x_max {
                self.x_max = p.x;
            }
            if p.y < self.y_min {
                self.y_min = p.y;
            }
            if p.y > self.y_max {
                self.y_max = p.y;
            }
        }
        self.num_points += points.len() as i32;
    }

    /// Adds a part of Point2Ds and measures to the ShapefileGeometry.
    pub fn add_partm(&mut self, points: &[Point2D], measures: &[f64]) {
        if points.len() != measures.len() {
            panic!("Error adding part to ShapefileGeometry. Points and measures array must be equal length.");
        }
        self.parts.push(self.points.len() as i32);
        let mut p: Point2D;
        let mut m: f64;
        for i in 0..points.len() {
            p = points[i];
            m = measures[i];
            self.points.push(p);
            if p.x < self.x_min {
                self.x_min = p.x;
            }
            if p.x > self.x_max {
                self.x_max = p.x;
            }
            if p.y < self.y_min {
                self.y_min = p.y;
            }
            if p.y > self.y_max {
                self.y_max = p.y;
            }
            if m < self.m_min {
                self.m_min = m;
            }
            if m > self.m_max {
                self.m_max = m;
            }
            self.m_array.push(m);
        }
        self.num_points += points.len() as i32;
        self.num_parts += 1i32;
    }

    /// Adds a part of Point2Ds, measures, and z-values to the ShapefileGeometry.
    pub fn add_partz(&mut self, points: &[Point2D], measures: &[f64], z_values: &[f64]) {
        if points.len() != measures.len() {
            panic!("Error adding part to ShapefileGeometry. Points and measures array must be equal length.");
        }
        if points.len() != z_values.len() {
            panic!(
                "Error adding part to ShapefileGeometry. Points and z array must be equal length."
            );
        }
        self.parts.push(self.points.len() as i32);
        let mut p: Point2D;
        let mut m: f64;
        let mut z: f64;
        for i in 0..points.len() {
            p = points[i];
            m = measures[i];
            z = z_values[i];
            self.points.push(p);
            if p.x < self.x_min {
                self.x_min = p.x;
            }
            if p.x > self.x_max {
                self.x_max = p.x;
            }
            if p.y < self.y_min {
                self.y_min = p.y;
            }
            if p.y > self.y_max {
                self.y_max = p.y;
            }
            if m < self.m_min {
                self.m_min = m;
            }
            if m > self.m_max {
                self.m_max = m;
            }
            self.m_array.push(m);
            if z < self.z_min {
                self.z_min = z;
            }
            if z > self.z_max {
                self.z_max = z;
            }
            self.z_array.push(z);
        }
        self.num_points += points.len() as i32;
        self.num_parts += 1i32;
    }

    pub fn get_bounding_box(&self) -> BoundingBox {
        BoundingBox::new(self.x_min, self.x_max, self.y_min, self.y_max)
    }

    /// Returns the length of the ShapefileGeometry, including the header, in bytes.
    pub fn get_length(&self) -> i32 {
        let mut ret = 4; //12; // 12 bytes in the record number, content length, and shape type

        ret += match self.shape_type {
            ShapeType::Null => 0i32,

            ShapeType::Point => 16i32,
            ShapeType::MultiPoint => 36i32 + self.num_points * 16i32,
            ShapeType::PolyLine | ShapeType::Polygon => {
                40i32 + self.num_parts * 4i32 + self.num_points * 16i32
            }

            ShapeType::PointM => 24i32,
            ShapeType::MultiPointM => 52i32 + self.num_points * 24i32,
            ShapeType::PolyLineM | ShapeType::PolygonM => {
                56i32 + self.num_parts * 4i32 + self.num_points * 24i32
            }

            ShapeType::PointZ => 32i32,
            ShapeType::MultiPointZ => 68i32 + self.num_points * 32i32,
            ShapeType::PolyLineZ | ShapeType::PolygonZ => {
                72i32 + self.num_parts * 4i32 + self.num_points * 32i32
            }
        };

        ret
    }

    /// Checks whether a point is within the hull of the geometry. If
    /// the geometry is not of POLYGON base ShapeType, the function
    /// will return false.
    pub fn is_point_within_hull(&self, point: &Point2D) -> bool {
        // see if it's a polygon
        if self.shape_type.base_shape_type() != ShapeType::Polygon {
            // it's not a polygon
            return false;
        }
        // get the first and last points in the hull.
        let last_point = if self.num_parts > 1 {
            self.parts[1] as usize
        } else {
            self.num_points as usize
        };

        point_in_poly(&point, &(self.points[0..last_point]))
    }

    /// Checks whether or not a part in a polygon is a hole.
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
            (self.parts[part_num as usize + 1] - 2i32) as usize
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
            if test_sign {
                // positive means counter-clockwise
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

            if area < 0f64 {
                // a positive area indicates counter-clockwise order
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
            x_min: f64::INFINITY,
            x_max: f64::NEG_INFINITY,
            y_min: f64::INFINITY,
            y_max: f64::NEG_INFINITY,
            num_parts: 0i32,
            num_points: 0i32,
            parts: vec![],
            points: vec![],
            z_min: f64::INFINITY,
            z_max: f64::NEG_INFINITY,
            z_array: vec![],
            m_min: f64::INFINITY,
            m_max: f64::NEG_INFINITY,
            m_array: vec![],
        }
    }
}

impl fmt::Display for ShapefileGeometry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!(
            "shape_type: {}
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
            self.m_array
        );
        write!(f, "{}", s)
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq)]
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
            _ => panic!("Unrecognized ShapeType"),
        }
    }

    pub fn to_int(&self) -> i32 {
        match self {
            ShapeType::Null => return 0,
            ShapeType::Point => return 1,
            ShapeType::PolyLine => return 3,
            ShapeType::Polygon => return 5,
            ShapeType::MultiPoint => return 8,
            ShapeType::PointZ => return 11,
            ShapeType::PolyLineZ => return 13,
            ShapeType::PolygonZ => return 15,
            ShapeType::MultiPointZ => return 18,
            ShapeType::PointM => return 21,
            ShapeType::PolyLineM => return 23,
            ShapeType::PolygonM => return 25,
            ShapeType::MultiPointM => return 28,
        }
    }

    pub fn base_shape_type(&self) -> ShapeType {
        match self {
            &ShapeType::Null => ShapeType::Null,
            &ShapeType::Point | &ShapeType::PointZ | &ShapeType::PointM => ShapeType::Point,
            &ShapeType::PolyLine | &ShapeType::PolyLineZ | &ShapeType::PolyLineM => {
                ShapeType::PolyLine
            }
            &ShapeType::Polygon | &ShapeType::PolygonZ | &ShapeType::PolygonM => ShapeType::Polygon,
            &ShapeType::MultiPoint | &ShapeType::MultiPointZ | &ShapeType::MultiPointM => {
                ShapeType::MultiPoint
            }
        }
    }

    pub fn dimension(&self) -> ShapeTypeDimension {
        match self {
            &ShapeType::Null
            | &ShapeType::MultiPoint
            | &ShapeType::Point
            | &ShapeType::Polygon
            | &ShapeType::PolyLine => ShapeTypeDimension::XY,
            &ShapeType::MultiPointM
            | &ShapeType::PointM
            | &ShapeType::PolygonM
            | &ShapeType::PolyLineM => ShapeTypeDimension::Measure,
            &ShapeType::MultiPointZ
            | &ShapeType::PointZ
            | &ShapeType::PolygonZ
            | &ShapeType::PolyLineZ => ShapeTypeDimension::Z,
        }
    }
}

#[repr(u16)]
#[derive(Copy, Clone, PartialEq)]
pub enum ShapeTypeDimension {
    XY,
    Measure,
    Z,
}

impl Default for ShapeType {
    fn default() -> ShapeType {
        ShapeType::Null
    }
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
