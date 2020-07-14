/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/08/2018
Last Modified: 30/08/2018
License: MIT
*/
use std::ops::{Add, Mul, Sub};
use std::{f64, fmt};

const EPSILON: f64 = std::f64::EPSILON; //f64::EPSILON * 2.0;

// const EPSILON_SQRD: f64 = EPSILON * EPSILON;

/// A 2-D point, with x and y fields.
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl fmt::Display for Point2D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("[{}, {}]", self.x, self.y);
        write!(f, "{}", s)
    }
}

impl Point2D {
    /// Creates a new Point2D,
    pub fn new(x: f64, y: f64) -> Point2D {
        Point2D { x: x, y: y }
    }

    /// Calculates the midpoint between two Point2Ds.
    pub fn midpoint(p1: &Point2D, p2: &Point2D) -> Point2D {
        Point2D::new((p1.x + p2.x) / 2f64, (p1.y + p2.y) / 2f64)
    }

    /// Calculates the centre point of a set of Point2Ds.
    pub fn centre_point(points: &[Point2D]) -> Point2D {
        let mut x = 0f64;
        let mut y = 0f64;

        for p in points {
            x += p.x;
            y += p.y;
        }

        x /= points.len() as f64;
        y /= points.len() as f64;

        Point2D::new(x, y)
    }

    /// Calculates the cross product
    pub fn cross(&self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Calculate Euclidean distance between the point and another.
    pub fn distance(&self, other: &Self) -> f64 {
        ((self.x - other.x) * (self.x - other.x) + (self.y - other.y) * (self.y - other.y)).sqrt()
    }

    /// Calculate squared Euclidean distance between the point and another.
    pub fn distance_squared(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Draw a horizontal line through this point, connect this point with the other,
    /// and measure the angle between these two lines.
    pub fn angle(&self, other: &Self) -> f64 {
        if self == other {
            0.0
        } else {
            (other.y - self.y).atan2(other.x - self.x)
        }
    }

    /// Calculates the magnitude sqrt(x^2 + y^2) of the point.
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn change_in_heading(previous: Point2D, current: Point2D, next: Point2D) -> f64 {
        let p1 = current - previous;
        let p2 = next - current;
        let mut ratio = (p1 * p2) / (p1.magnitude() * p2.magnitude());
        // check for rounding errors; -1.0 < ratio < 1.0
        if ratio > 1.0 {
            ratio = 1.0;
        }
        if ratio < -1.0 {
            ratio = -1.0;
        }
        if current.is_left(&previous, &next) >= 0.0 {
            ratio.acos()
        } else {
            -1.0 * ratio.acos()
        }
    }

    pub fn sin_cos(&self) -> (f64, f64) {
        let mag = self.magnitude();
        (self.y / mag, self.x / mag)
    }

    pub fn rotate(&self, theta: f64) -> Point2D {
        let cosine = theta.cos();
        let sine = theta.sin();
        let x_cos_theta = self.x * cosine;
        let x_sin_theta = self.x * sine;
        let y_cos_theta = self.y * cosine;
        let y_sin_theta = self.y * sine;
        let x1 = x_cos_theta - y_sin_theta;
        let y1 = x_sin_theta + y_cos_theta;
        Point2D::new(x1, y1)
    }

    pub fn translate(&self, delta_x: f64, delta_y: f64) -> Point2D {
        Point2D::new(self.x + delta_x, self.y + delta_y)
    }

    pub fn direction(&self, p1: &Self, p2: &Self) -> Direction {
        let v1 = *p1 - *self;
        let v2 = *p2 - *self;
        let x1 = v1.x;
        let x2 = v2.x;
        let y1 = v1.y;
        let y2 = v2.y;
        let det = x1 * y2 - y1 * x2;
        if det < 0.0 {
            Direction::Right
        } else if det > 0.0 {
            Direction::Left
        } else {
            Direction::Ahead
        }
    }

    pub fn orient(&self, q: &Self, r: &Self) -> bool {
        (q.y - self.y) * (r.x - q.x) - (q.x - self.x) * (r.y - q.y) < 0.0
    }

    pub fn circumdelta(&self, b: &Self, c: &Self) -> (f64, f64) {
        let dx = b.x - self.x;
        let dy = b.y - self.y;
        let ex = c.x - self.x;
        let ey = c.y - self.y;

        let bl = dx * dx + dy * dy;
        let cl = ex * ex + ey * ey;
        let d = 0.5 / (dx * ey - dy * ex);

        let x = (ey * bl - dy * cl) * d;
        let y = (dx * cl - ex * bl) * d;
        (x, y)
    }

    pub fn circumradius2(&self, b: &Self, c: &Self) -> f64 {
        let (x, y) = self.circumdelta(b, c);
        x * x + y * y
    }

    pub fn circumcenter(&self, b: &Self, c: &Self) -> Self {
        let (x, y) = self.circumdelta(b, c);
        Self {
            x: self.x + x,
            y: self.y + y,
        }
    }

    pub fn in_circle(&self, b: &Self, c: &Self, p: &Self) -> bool {
        let dx = self.x - p.x;
        let dy = self.y - p.y;
        let ex = b.x - p.x;
        let ey = b.y - p.y;
        let fx = c.x - p.x;
        let fy = c.y - p.y;

        let ap = dx * dx + dy * dy;
        let bp = ex * ex + ey * ey;
        let cp = fx * fx + fy * fy;

        dx * (ey * cp - bp * fy) - dy * (ex * cp - bp * fx) + ap * (ex * fy - ey * fx) < 0.0
    }

    pub fn nearly_equals(&self, other: &Self) -> bool {
        // (self.x - other.x).abs() <= EPSILON && (self.y - other.y).abs() <= EPSILON
        // ((self.x - other.x) * (self.x - other.x) + (self.y - other.y) * (self.y - other.y))
        //     <= EPSILON_SQRD
        (self.x - other.x).hypot(self.y - other.y) <= 1e-10
    }

    /// Tests if a point is Left|On|Right of an infinite line,
    /// based on http://geomalgorithms.com/a03-_inclusion.html.
    ///
    /// Input:  two points, p0 and p1, on the line
    ///
    /// Return: > 0 for self left of the line through p0 and p1
    ///         = 0 for self on the line through p0 and p1
    ///         < 0 for self right of the line through p0 and p1
    pub fn is_left(&self, p0: &Point2D, p1: &Point2D) -> f64 {
        (p1.x - p0.x) * (self.y - p0.y) - (self.x - p0.x) * (p1.y - p0.y)
    }

    pub fn is_between(&self, a: &Self, b: &Self) -> bool {
        let crossproduct = (self.y - a.y) * (b.x - a.x) - (self.x - a.x) * (b.y - a.y);
        if crossproduct > -EPSILON && crossproduct < EPSILON {
            if self.x >= a.x.min(b.x) && self.x <= a.x.max(b.x) {
                if self.y >= a.y.min(b.y) && self.y <= a.y.max(b.y) {
                    return true;
                }
            }
        }
        false
    }

    pub fn fix_precision(&self, decimal_places: usize) -> Point2D {
        let multiplier = 10f64.powf(decimal_places as f64);
        let x = (self.x * multiplier).round() / multiplier;
        let y = (self.y * multiplier).round() / multiplier;
        Point2D::new(x, y)
    }
}

impl Eq for Point2D {}

// impl PartialEq for Point2D {
//     fn eq(&self, other: &Self) -> bool {
//         self.x == other.x && self.y == other.y
//     }
// }

// impl PartialOrd for Point2D {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         // Some(other.priority.cmp(&self.priority))
//         self.y.partial_cmp(&other.y)
//     }
// }

// impl Ord for Point2D {
//     fn cmp(&self, other: &Point2D) -> Ordering {
//         // other.priority.cmp(&self.priority)
//         let ord = self.partial_cmp(other).unwrap();
//         match ord {
//             Ordering::Greater => Ordering::Greater,
//             Ordering::Less => Ordering::Less,
//             Ordering::Equal => ord,
//         }
//     }
// }

// impl PartialOrd for Point2D {
//     fn partial_cmp(&self, other: &Point2D) -> Option<Ordering> {
//         Some(self.y.cmp(other.y))
//     }
// }

// impl Ord for DefinitelyANumber {
//     fn cmp(&self, other: &DefinitelyANumber) -> Ordering {
//         self.0.partial_cmp(&other.0).expect("A number that can't be NaN was NaN")
//     }
// }

impl Add for Point2D {
    type Output = Point2D;
    fn add(self, other: Self) -> Point2D {
        Point2D {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point2D {
    type Output = Point2D;
    fn sub(self, other: Self) -> Point2D {
        Point2D {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

// dot product
impl Mul for Point2D {
    type Output = f64;
    fn mul(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }
}

#[derive(Debug, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Ahead,
}
