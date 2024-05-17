/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/10/2018
Last Modified: 18/10/2018
License: MIT
*/

use super::{BoundingBox, Point2D};
use std::ops::Index;

/// A storage container for multiple related polylines.
#[derive(Default, Clone, Debug)]
pub struct MultiPolyline {
    parts: Vec<Polyline>,
    pub id: usize,
    bounding_box: BoundingBox,
}

impl Index<usize> for MultiPolyline {
    type Output = Polyline;

    fn index<'a>(&'a self, index: usize) -> &'a Polyline {
        &self.parts[index]
    }
}

impl MultiPolyline {
    /// Creates a new MultiPolyline
    pub fn new(id: usize) -> MultiPolyline {
        MultiPolyline {
            parts: vec![],
            bounding_box: BoundingBox::default(),
            id: id,
        }
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn push(&mut self, polyline: &Polyline) {
        self.parts.push(polyline.clone());
        self.bounding_box.expand_to(polyline.get_bounding_box());
    }

    pub fn get_bounding_box(&self) -> BoundingBox {
        self.bounding_box
    }
}

#[derive(Default, Clone, Debug)]
pub struct Polyline {
    pub vertices: Vec<Point2D>,
    pub source_file: usize,
    pub id: usize,
    pub split_points: Vec<(f64, Point2D)>,
}

impl PartialEq for Polyline {
    fn eq(&self, other: &Self) -> bool {
        // Equality is based on vertices coordinates only.
        // The id, source_file and split points don't impact eqality.
        // This is because equality is often used to identify duplicate polylines.
        if self.len() == other.len() {
            // polylines are considered equal even if they are reversed in order
            // let (starting_point_same, reversed) = if self[0].nearly_equals(&other[0]) {
            //     (true, false)
            // } else if self[0].nearly_equals(&other[other.len() - 1]) {
            //     (true, true)
            // } else {
            //     (false, false)
            // };

            // if starting_point_same {
            //     if !reversed {
            //         for p in 1..self.len() {
            //             if !(self[p].nearly_equals(&other[p])) {
            //                 return false;
            //             }
            //         }
            //         return true;
            //     } else {
            //         for p in 1..self.len() {
            //             if !(self[p].nearly_equals(&other[other.len() - 1 - p])) {
            //                 return false;
            //             }
            //         }
            //         return true;
            //     }
            // }

            // if self.first_vertex() == other.first_vertex() {
            //     for p in 1..self.len() {
            //         if self.get(p) != other.get(p) {
            //             return false;
            //         }
            //     }
            //     return true;
            // }
            return false;
        }
        false
    }
}

impl Index<usize> for Polyline {
    type Output = Point2D;

    fn index<'a>(&'a self, index: usize) -> &'a Point2D {
        &self.vertices[index]
    }
}

impl Polyline {
    /// Creates a new Polyline from vertices
    pub fn new(vertices: &[Point2D], id: usize) -> Polyline {
        Polyline {
            vertices: vertices.to_vec(),
            source_file: 0,
            id: id,
            split_points: vec![],
        }
    }

    /// Creates a new empty Polyline
    pub fn new_empty(id: usize) -> Polyline {
        Polyline {
            vertices: vec![],
            source_file: 0,
            id: id,
            split_points: vec![],
        }
    }

    /// Creates a new Polyline with capacity
    pub fn new_with_capacity(id: usize, capacity: usize) -> Polyline {
        Polyline {
            vertices: Vec::with_capacity(capacity),
            source_file: 0,
            id: id,
            split_points: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.split_points.clear();
    }

    /// returns the number of vertices
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    pub fn num_splits(&self) -> usize {
        self.split_points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.len() == 0
    }

    /// Returns the feature geometric length.
    pub fn length(&self) -> f64 {
        let mut ret = 0f64;
        for a in 0..self.len() - 1 {
            ret += self[a].distance(&self[a + 1]);
        }
        ret
    }

    pub fn get(&self, index: usize) -> Point2D {
        self.vertices[index]
    }

    pub fn first_vertex(&self) -> Point2D {
        self.vertices[0]
    }

    pub fn last_vertex(&self) -> Point2D {
        self.vertices[self.vertices.len() - 1]
    }

    /// Inserts a point vertex at the end of the line.
    pub fn push(&mut self, v: Point2D) {
        self.vertices.push(v);
    }

    /// Inserts a point vertex at a specific index.
    pub fn insert(&mut self, index: usize, v: Point2D) {
        if index <= self.len() {
            self.vertices.insert(index, v);
        }
    }

    /// Removes a point vertex at a specified index.
    pub fn remove(&mut self, index: usize) {
        if index <= self.len() {
            self.vertices.remove(index);
        }
    }

    /// Closes the line by pushing a duplicate of the first vertex.
    pub fn close_line(&mut self) {
        let v = self.first_vertex().clone();
        self.push(v);
    }

    pub fn get_split_point(&self, index: usize) -> (f64, Point2D) {
        self.split_points[index]
    }

    /// Inserts a split point into the polyline, which can be used to eventually break
    /// the original polyline into two new lines. The `position` is a floating point (f64)
    /// value representing the position along the polyline for the new end-point insertion.
    /// For example, a position of 3.5 means that an end point will be inserted halfway
    /// along the line segment connecting vertex 3 and vertex 4. Notice that integer values
    /// have the effect of inserting a duplicate vertex and a zero-length segment, which
    /// in most applications is likely very undesirable.
    ///
    /// Split points cannot be inserted at line endpoints.
    pub fn insert_split_point(&mut self, position: f64, point: Point2D) {
        if position > 0f64 && position < (self.len() - 1) as f64 {
            self.split_points.push((position, point));
        }
    }

    pub fn remove_split_point(&mut self, index: usize) {
        if index <= self.split_points.len() {
            self.split_points.remove(index);
        }
    }

    pub fn split(&mut self) -> Vec<Polyline> {
        // first, sort the split points by position, first to last
        self.split_points
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let mut ret: Vec<Polyline> = Vec::with_capacity(self.split_points.len() + 1);
        if self.num_splits() > 0 {
            let mut line: Vec<Point2D> = vec![];
            let mut next_split = 0;
            let mut upper_index = self.split_points[next_split].0.floor() as usize;

            let mut is_integer = if self.split_points[next_split].0
                - self.split_points[next_split].0.floor()
                != 0f64
            {
                false
            } else {
                true
            };

            let mut i = 0;
            while i < self.len() {
                if i <= upper_index {
                    line.push(self.vertices[i]);
                } else {
                    if !is_integer {
                        line.push(self.split_points[next_split].1);
                    }
                    ret.push(Polyline::new(&line, self.id));
                    line.clear();
                    line.push(self.split_points[next_split].1);
                    next_split += 1;
                    i -= 1;
                    if next_split < self.num_splits() {
                        upper_index = self.split_points[next_split].0.floor() as usize;

                        is_integer = if self.split_points[next_split].0
                            - self.split_points[next_split].0.floor()
                            != 0f64
                        {
                            false
                        } else {
                            true
                        };
                    } else if next_split == self.num_splits() {
                        upper_index = self.len() - 1;
                    }
                }
                i += 1;
            }
            // push the last polyline
            ret.push(Polyline::new(&line, self.id));

            for a in 0..ret.len() {
                ret[a].source_file = self.source_file;
            }
            return ret;
        }
        let mut pl = Polyline::new(&(self.vertices), self.id);
        pl.source_file = self.source_file;
        ret.push(pl);
        ret
    }

    // pub fn snap_to_line(&self, other: &mut Self, precision: f64) {
    //     // let box1 = self.get_bounding_box();
    //     // let box2 = other.get_bounding_box();
    //     // if box1.nearly_overlaps(box2, precision) {
    //     for a in 0..self.len() {
    //         for b in 0..other.len() {
    //             if self.vertices[a].distance(&other.vertices[b]) < precision {
    //                 other.vertices[b] = self.vertices[a].clone();
    //             }
    //         }
    //     }
    //     // }
    // }

    pub fn get_bounding_box(&self) -> BoundingBox {
        BoundingBox::from_points(&self.vertices)
    }

    pub fn nearly_equals(&self, other: &Self, precision: f64) -> bool {
        // Equality is based on vertices coordinates only.
        // The id, source_file and split points don't impact eqality.
        // This is because equality is often used to identify duplicate polylines.
        let prec = precision * precision;
        if self.len() == other.len() {
            // polylines are considered equal even if they are reversed in order
            let (starting_point_same, reversed) = if self[0].distance_squared(&other[0]) <= prec {
                (true, false)
            } else if self[0].distance_squared(&other[other.len() - 1]) <= prec {
                (true, true)
            } else {
                (false, false)
            };
            if starting_point_same {
                if !reversed {
                    for p in 1..self.len() {
                        if self[p].distance_squared(&other[p]) > prec {
                            return false;
                        }
                    }
                    return true;
                } else {
                    for p in 1..self.len() {
                        if self[p].distance_squared(&other[other.len() - 1 - p]) > prec {
                            return false;
                        }
                    }
                    return true;
                }
            }
        }
        false
    }
}

// #[cfg(test)]
// mod test {
//     use super::Polyline;
//     use crate::structures::Point2D;

//     #[test]
//     fn test_polyline_split() {
//         let mut pl = Polyline::new(
//             &vec![
//                 Point2D::new(0.0, 0.0),
//                 Point2D::new(10.0, 10.0),
//                 Point2D::new(12.0, 6.0),
//                 Point2D::new(6.0, 0.0),
//             ],
//             1,
//         );
//         pl.insert_split_point(0.5, Point2D::new(5.0, 5.0));
//         pl.insert_split_point(2.5, Point2D::new(9.0, 3.0));
//         let new_polylines = pl.split();
//         let new_polyline_should_be = vec![
//             Polyline::new(&vec![Point2D::new(0.0, 0.0), Point2D::new(5.0, 5.0)], 1),
//             Polyline::new(
//                 &vec![
//                     Point2D::new(5.0, 5.0),
//                     Point2D::new(10.0, 10.0),
//                     Point2D::new(12.0, 6.0),
//                     Point2D::new(9.0, 3.0),
//                 ],
//                 1,
//             ),
//             Polyline::new(&vec![Point2D::new(9.0, 3.0), Point2D::new(6.0, 0.0)], 1),
//         ];
//         assert_eq!(new_polylines, new_polyline_should_be);
//     }
// }
