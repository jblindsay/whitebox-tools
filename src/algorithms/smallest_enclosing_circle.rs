/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/09/2018
Last Modified: 27/09/2018
License: MIT

NOTES: The logic of this algorithm is based on
       https://www.nayuki.io/res/smallest-enclosing-circle/SmallestEnclosingCircle.cs and
       https://www.nayuki.io/res/smallest-enclosing-circle/SmallestEnclosingCircle.cpp
*/
use crate::structures::{Circle, Point2D};
use rand::prelude::*;

/// Returns the smallest circle that encloses all the given points. Runs in expected O(n) time,
/// randomized.
/// Note: If 0 points are given, a circle of radius -1f64 is returned. If 1 point is given, a circle of
/// radius 0f64 is returned.
/// Initially: No boundary points known
pub fn smallest_enclosing_circle(points: &[Point2D]) -> Circle {
    // Clone list to preserve the caller's data, do Durstenfeld shuffle
    let mut shuffled: Vec<Point2D> = points.to_vec();

    let mut rng = thread_rng();
    for i in (1..shuffled.len()).rev() {
        let j = rng.gen_range(0, i + 1);
        let temp = shuffled[i];
        shuffled[i] = shuffled[j];
        shuffled[j] = temp;
    }

    // Progressively add points to circle or recompute circle
    let mut c = Circle::new(Point2D::new(0f64, 0f64), -1f64);
    for i in 0..shuffled.len() {
        let p = shuffled[i];
        if c.radius < 0f64 || !c.contains(p) {
            c = make_circle_one_point(&shuffled[0..i + 1], p);
        }
    }
    c
}

// One boundary point known
fn make_circle_one_point(points: &[Point2D], p: Point2D) -> Circle {
    let mut c = Circle::new(p, 0f64);
    for i in 0..points.len() {
        let q = points[i];
        if !c.contains(q) {
            c = if c.radius == 0f64 {
                make_diameter(p, q)
            } else {
                make_circle_two_points(&points[0..i + 1], p, q)
            };
        }
    }
    c
}

// Two boundary points known
fn make_circle_two_points(points: &[Point2D], p: Point2D, q: Point2D) -> Circle {
    let circ = make_diameter(p, q);
    let mut left = Circle::new(Point2D::new(0f64, 0f64), -1f64);
    let mut right = Circle::new(Point2D::new(0f64, 0f64), -1f64);

    // For each point not in the two-point circle
    let pq = q - p;
    for r in points {
        if circ.contains(*r) {
            continue;
        }

        // Form a circumcircle and classify it on left or right side
        let cross = pq.cross(*r - p);
        let c = make_circumcircle(p, q, *r);
        if c.radius < 0f64 {
            continue;
        } else if cross > 0f64
            && (left.radius < 0f64 || pq.cross(c.center - p) > pq.cross(left.center - p))
        {
            left = c;
        } else if cross < 0f64
            && (right.radius < 0f64 || pq.cross(c.center - p) < pq.cross(right.center - p))
        {
            right = c;
        }
    }

    // Select which circle to return
    if left.radius < 0f64 && right.radius < 0f64 {
        return circ;
    } else if left.radius < 0f64 {
        return right;
    } else if right.radius < 0f64 {
        return left;
    }
    if left.radius <= right.radius {
        return left;
    }
    right
}

fn make_diameter(a: Point2D, b: Point2D) -> Circle {
    let c = Point2D::new((a.x + b.x) / 2f64, (a.y + b.y) / 2f64);
    Circle::new(c, (c.distance(&a)).max(c.distance(&b)))
}

fn make_circumcircle(a: Point2D, b: Point2D, c: Point2D) -> Circle {
    // Mathematical algorithm from Wikipedia: Circumscribed circle
    let ox = ((a.x.min(b.x)).min(c.x) + (a.x.min(b.x)).max(c.x)) / 2f64;
    let oy = ((a.y.min(b.y)).min(c.y) + (a.y.min(b.y)).max(c.y)) / 2f64;
    let ax = a.x - ox;
    let ay = a.y - oy;
    let bx = b.x - ox;
    let by = b.y - oy;
    let cx = c.x - ox;
    let cy = c.y - oy;
    let d = (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by)) * 2f64;
    if d == 0f64 {
        return Circle::new(Point2D::new(0f64, 0f64), -1f64);
    }
    let x = ((ax * ax + ay * ay) * (by - cy)
        + (bx * bx + by * by) * (cy - ay)
        + (cx * cx + cy * cy) * (ay - by))
        / d;
    let y = ((ax * ax + ay * ay) * (cx - bx)
        + (bx * bx + by * by) * (ax - cx)
        + (cx * cx + cy * cy) * (bx - ax))
        / d;
    let p = Point2D::new(ox + x, oy + y);
    let r = (p.distance(&a).max(p.distance(&b))).max(p.distance(&c));

    Circle::new(p, r)
}

#[cfg(test)]
mod test {
    use super::smallest_enclosing_circle;
    use crate::structures::{Circle, Point2D};
    #[test]
    fn test_smallest_enclosing_circle() {
        // make a square
        let side_length = 2f64;
        let mut points: Vec<Point2D> = Vec::new();
        points.push(Point2D::new(0f64, 0f64)); // origin
        points.push(Point2D::new(side_length, 0f64));
        points.push(Point2D::new(side_length, side_length));
        points.push(Point2D::new(0f64, side_length));

        // add some interior points
        points.push(Point2D::new(side_length * 0.5, side_length * 0.5));
        points.push(Point2D::new(side_length * 0.25, side_length * 0.7));
        points.push(Point2D::new(side_length * 0.1, side_length * 0.9));

        let circle = smallest_enclosing_circle(&points);

        let centre = Point2D::new(side_length / 2f64, side_length / 2f64);
        let r = centre.distance(&Point2D::new(side_length, side_length));
        assert_eq!(circle, Circle::new(centre, r));
    }
}
