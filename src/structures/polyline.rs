/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/10/2018
Last Modified: 18/10/2018
License: MIT
*/

use std::ops::Index;
use structures::{BoundingBox, Point2D};

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Polyline {
    pub vertices: Vec<Point2D>,
    pub id: usize,
    pub split_points: Vec<(f64, Point2D)>,
}

impl Index<usize> for Polyline {
    type Output = Point2D;

    fn index<'a>(&'a self, index: usize) -> &'a Point2D {
        &self.vertices[index]
    }
}

impl Polyline {
    /// Creates a new Circle,
    pub fn new(vertices: &[Point2D], id: usize) -> Polyline {
        Polyline {
            vertices: vertices.clone().to_vec(),
            id: id,
            split_points: vec![],
        }
    }

    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    pub fn num_splits(&self) -> usize {
        self.split_points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.len() == 0
    }

    pub fn get(&self, index: usize) -> Point2D {
        self.vertices[index]
    }

    pub fn start_vertex(&self) -> Point2D {
        self.vertices[0]
    }

    pub fn end_vertex(&self) -> Point2D {
        self.vertices[self.vertices.len() - 1]
    }

    pub fn insert(&mut self, index: usize, v: Point2D) {
        if index <= self.len() {
            self.vertices.insert(index, v);
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index <= self.len() {
            self.vertices.remove(index);
        }
    }

    pub fn get_split_point(&self, index: usize) -> (f64, Point2D) {
        self.split_points[index]
    }

    /// Inserts a split point into the polyline, which can be used to eventually break
    /// the orignal polyline into two new lines. The `position` is a floating point (f64)
    /// value representing the position along the polyline for the new end-point insertion.
    /// For example, a position of 3.5 means that an end point will be inserted halfway
    /// along the line segment connecting vertex 3 and vertex 4. Notice that integer values
    /// have the effect of inserting a duplicate vertex and a zero-length segment, which
    /// in most applications is likely very undesirable.
    pub fn insert_split_point(&mut self, position: f64, point: Point2D) {
        if position < (self.len() - 1) as f64 {
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
            let mut i = 0;
            while i < self.len() {
                if i <= upper_index {
                    line.push(self.vertices[i]);
                } else {
                    line.push(self.split_points[next_split].1);
                    ret.push(Polyline::new(&line, self.id));
                    line.clear();
                    line.push(self.split_points[next_split].1);
                    next_split += 1;
                    i -= 1;
                    if next_split < self.num_splits() {
                        upper_index = self.split_points[next_split].0.floor() as usize;
                    } else if next_split == self.num_splits() {
                        upper_index = self.len() - 1;
                    }
                }
                i += 1;
            }
            // push the last polyline
            ret.push(Polyline::new(&line, self.id));
            return ret;
        }

        ret.push(Polyline::new(&(self.vertices), self.id));
        ret
    }

    pub fn get_bounding_box(&self) -> BoundingBox {
        BoundingBox::from_points(&self.vertices)
    }
}

#[cfg(test)]
mod test {
    use super::Polyline;
    use structures::Point2D;

    #[test]
    fn test_polyline_split() {
        let mut pl = Polyline::new(
            &vec![
                Point2D::new(0.0, 0.0),
                Point2D::new(10.0, 10.0),
                Point2D::new(12.0, 6.0),
                Point2D::new(6.0, 0.0),
            ],
            1,
        );
        pl.insert_split_point(0.5, Point2D::new(5.0, 5.0));
        pl.insert_split_point(2.5, Point2D::new(9.0, 3.0));
        let new_polylines = pl.split();
        let new_polyline_should_be = vec![
            Polyline::new(&vec![Point2D::new(0.0, 0.0), Point2D::new(5.0, 5.0)], 1),
            Polyline::new(
                &vec![
                    Point2D::new(5.0, 5.0),
                    Point2D::new(10.0, 10.0),
                    Point2D::new(12.0, 6.0),
                    Point2D::new(9.0, 3.0),
                ],
                1,
            ),
            Polyline::new(&vec![Point2D::new(9.0, 3.0), Point2D::new(6.0, 0.0)], 1),
        ];
        assert_eq!(new_polylines, new_polyline_should_be);
    }
}
