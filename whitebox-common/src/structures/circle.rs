/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/09/2018
Last Modified: 27/09/2018
License: MIT
*/

use super::Point2D;

const MULTIPLICATIVE_EPSILON: f64 = 1f64 + 1e-14;

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Circle {
    pub center: Point2D, // Center
    pub radius: f64,     // Radius
}

impl Circle {
    /// Creates a new Circle,
    pub fn new(center: Point2D, radius: f64) -> Circle {
        Circle {
            center: center,
            radius: radius,
        }
    }

    pub fn contains(&self, p: Point2D) -> bool {
        self.center.distance(&p) <= self.radius * MULTIPLICATIVE_EPSILON
    }
}
