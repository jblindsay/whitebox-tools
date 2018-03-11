/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: July 17, 2017
License: MIT
*/

/* 
Eventually this will be used to support multiple vector formats but
for now it's just Shapefiles.
*/

use std::fmt;

// private sub-module defined in other files
mod shapefile;

// exports identifiers from private sub-modules in the current module namespace
pub use self::shapefile::Shapefile;
pub use self::shapefile::ShapeType;

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


/////////////////////////////////////////////////////////////////////////////
// The following is based on http://geomalgorithms.com/a03-_inclusion.html //
/////////////////////////////////////////////////////////////////////////////

// is_left: tests if a point is Left|On|Right of an infinite line.
//    Input:  three points P0, P1, and P2
//    Return: >0 for P2 left of the line through P0 and P1
//            =0 for P2  on the line
//            <0 for P2  right of the line
// [#inline_always]
pub fn is_left(p0: &Point2D, p1: &Point2D, p2: &Point2D) -> f64 {
    (p1.x - p0.x) * (p2.y - p0.y) - (p2.x -  p0.x) * (p1.y - p0.y)
}

// point_in_poly: winding number test for a point in a polygon
//    Input:   p = a point,
//             v[] = vertex points of a polygon v[n+1] with v[n]=v[0]
//    Return:  wn = the winding number (=0 only when p is outside)
pub fn point_in_poly(p: &Point2D, v: &Vec<Point2D>) -> bool {
    let mut wn = 0i32;
    // loop through all edges of the polygon
    for i in 0..v.len()-1 { // edge from v[i] to v[i+1]
        if v[i].y <= p.y { // start y <= p.y
            if v[i+1].y  > p.y { // an upward crossing
                 if is_left(&v[i], &v[i+1], &p) > 0f64 { // p left of edge
                     wn += 1i32; // have a valid up intersect
                 }
            }
        } else { // start y > p.y (no test needed)
            if v[i+1].y  <= p.y { // a downward crossing
                 if is_left(&v[i], &v[i+1], &p) < 0f64 {  // p right of edge
                     wn -= 1i32; // have a valid down intersect
                 }
            }
        }
    }
    wn > 0i32
}