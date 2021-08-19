/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/10/2018
Last Modified: 15/10/2018
License: MIT
*/

// use std::f64::EPSILON;
use super::{BoundingBox, Point2D};

const EPSILON: f64 = std::f64::EPSILON; //f64::EPSILON * 2.0;

/// A data structure to hold line segments, defined by
/// starting and ending points.
#[derive(Default, Copy, Clone, Debug)]
pub struct LineSegment {
    pub p1: Point2D,
    pub p2: Point2D,
}

impl PartialEq for LineSegment {
    fn eq(&self, other: &Self) -> bool {
        (self.p1.nearly_equals(&other.p1) && self.p2.nearly_equals(&other.p2))
            || (self.p1.nearly_equals(&other.p2) && self.p2.nearly_equals(&other.p1))
    }
}

impl LineSegment {
    /// Creates a new LineSegment.
    pub fn new(p1: Point2D, p2: Point2D) -> LineSegment {
        LineSegment { p1, p2 }
    }

    /// Finds intersections between two line segments. Notice that segments
    /// can intersect at points or line segments. The function returns a
    /// line segment, but when the two test segments intersect at a point
    /// instead, output.p1 = output.p2.
    ///
    /// Based on https://en.wikipedia.org/wiki/Line%E2%80%93line_intersection
    /// and https://martin-thoma.com/how-to-check-if-two-line-segments-intersect/
    pub fn get_intersection(&self, other: &Self) -> Option<LineSegment> {
        if self == other {
            return Some(self.clone());
        }
        if self.p1 == self.p2 || other.p1 == other.p2 {
            return None;
        }
        // if (self.p1.distance_squared(&other.p1) + self.p2.distance_squared(&other.p2)) < 0.0001f64
        //     || (self.p1.distance_squared(&other.p2) + self.p2.distance_squared(&other.p1))
        //         < 0.0001f64
        // {
        //     return Some(self.clone());
        // }
        let box1 = self.get_bounding_box();
        let box2 = other.get_bounding_box();
        if box1.overlaps(box2) {
            let denom = (self.p1.x - self.p2.x) * (other.p1.y - other.p2.y)
                - (self.p1.y - self.p2.y) * (other.p1.x - other.p2.x);
            if denom != 0f64 {
                let t = ((self.p1.x - other.p1.x) * (other.p1.y - other.p2.y)
                    - (self.p1.y - other.p1.y) * (other.p1.x - other.p2.x))
                    / denom;

                let u = -((self.p1.x - self.p2.x) * (self.p1.y - other.p1.y)
                    - (self.p1.y - self.p2.y) * (self.p1.x - other.p1.x))
                    / denom;

                if t >= 0f64 && t <= 1f64 && u >= 0f64 && u <= 1f64 {
                    let p = Point2D::new(
                        self.p1.x + t * (self.p2.x - self.p1.x),
                        self.p1.y + t * (self.p2.y - self.p1.y),
                    );
                    return Some(LineSegment::new(p, p));
                }
            }

            // are the lines coincident?
            if self.is_point_on_line(other.p1) {
                // what is the coincident interval?
                let mut contained = [false; 4];
                contained[0] = self.p1.is_between(&other.p1, &other.p2);
                contained[1] = other.p1.is_between(&self.p1, &self.p2);
                contained[2] = self.p2.is_between(&other.p1, &other.p2);
                contained[3] = other.p2.is_between(&self.p1, &self.p2);

                // two of the above should be true
                let mut i = 4;
                let mut j = 4;
                for a in 0..4 {
                    if contained[a] {
                        i = a;
                        break;
                    }
                }
                for a in (0..4).rev() {
                    if contained[a] {
                        j = a;
                        break;
                    }
                }
                let p1 = if i == 0 {
                    self.p1
                } else if i == 1 {
                    other.p1
                } else if i == 2 {
                    self.p2
                } else if i == 3 {
                    other.p2
                } else {
                    // panic!("Error encountered in finding endpoints of overlapping segments.")
                    return None;
                };

                let p2 = if j == 0 {
                    self.p1
                } else if j == 1 {
                    other.p1
                } else if j == 2 {
                    self.p2
                } else if j == 3 {
                    other.p2
                } else {
                    // panic!("Error encountered in finding endpoints of overlapping segments.")
                    return None;
                };

                return Some(LineSegment::new(p1, p2));
            }
        }

        // the lines are parallel but not coincident
        None
    }

    pub fn get_bounding_box(&self) -> BoundingBox {
        BoundingBox::from_two_points(self.p1, self.p2)
    }

    // pub fn does_segment_intersect_other(&self, other: &Self) -> bool {
    //     let box1 = BoundingBox::from_points(&self.points);
    //     let box2 = BoundingBox::from_points(&other.points);
    //     box1.overlaps(box2)
    //         && self.line_segment_touches_or_crosses_line(other)
    //         && other.line_segment_touches_or_crosses_line(self)
    // }

    // /// Check if line segment first touches or crosses the line that is
    // /// defined by line segment second.
    // fn line_segment_touches_or_crosses_line(&self, other: &Self) -> bool {
    //     return self.is_point_on_line(other.p1)
    //         || self.is_point_on_line(other.p2)
    //         || (self.is_point_right_of_line(other.p1)
    //             ^ self.is_point_right_of_line(other.p2));
    // }

    /// Checks if a Point is on a line defined by two points. Notice that
    /// this test whether the test point lies on the infinite line passing
    /// through the test line segment, and not on the line segment itself.
    fn is_point_on_line(&self, p: Point2D) -> bool {
        let r = (self.p2 - self.p1).cross(p - self.p1);
        r.abs() < EPSILON
    }

    pub fn is_point_on_line_segment(&self, p: Point2D) -> bool {
        if self.is_point_on_line(p) {
            if p.x >= self.p1.x.min(self.p2.x) {
                if p.x <= self.p1.x.max(self.p2.x) {
                    if p.y >= self.p1.y.min(self.p2.y) {
                        if p.y <= self.p1.y.max(self.p2.y) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn dist_to_segment_squared(&self, p: Point2D) -> f64 {
        let l2 = self.p1.distance_squared(&self.p2);
        if l2 == 0.0 { return p.distance_squared(&self.p1) };
        let mut t = ((p.x - self.p1.x) * (self.p2.x - self.p1.x) + (p.y - self.p1.y) * (self.p2.y - self.p1.y)) / l2;
        t = 0f64.max(1f64.min(t));
        p.distance_squared(
            &Point2D::new(
                self.p1.x + t * (self.p2.x - self.p1.x),
                self.p1.y + t * (self.p2.y - self.p1.y)
            )
        )
    }

    pub fn dist_to_segment(&self, p: Point2D) -> f64 {
        self.dist_to_segment_squared(p).sqrt()
    }

    // fn is_point_right_of_line(&self, p: Point2D) -> bool {
    //     (self.p2 - self.p1).cross(p - self.p1) < 0f64
    // }
}
