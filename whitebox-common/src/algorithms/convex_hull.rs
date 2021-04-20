/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/08/2018
Last Modified: 30/08/2018
License: MIT
*/
use crate::structures::{Direction, Point2D};
use std::cmp::Ordering;

/// Returns the convex hull of a vector of Point2D in counter-clockwise order.
pub fn convex_hull(points: &mut Vec<Point2D>) -> Vec<Point2D> {
    let mut hull: Vec<Point2D> = Vec::new();
    sort_points(points);
    hull.push(points[0]);
    hull.push(points[1]);
    for i in 2..points.len() {
        loop {
            let m1 = hull.len() - 1;
            let m0 = m1 - 1;
            let direction = hull[m0].direction(&hull[m1], &points[i]);
            match direction {
                Direction::Left => {
                    hull.push(points[i]);
                    break;
                }
                Direction::Ahead => {
                    hull.pop();
                    hull.push(points[i]);
                    break;
                }
                _ => {
                    hull.pop();
                    ()
                }
            }
        }
    }
    hull
}

// sort by angle to head
fn sort_points(points: &mut Vec<Point2D>) {
    find_lowest_point(points);
    let head = points[0];
    points.sort_by(|a, b| {
        // head always comes first.
        if a == &head {
            return Ordering::Less;
        }
        if b == &head {
            return Ordering::Greater;
        }
        let area = (a.x - head.x) * (b.y - head.y) - (b.x - head.x) * (a.y - head.y);

        if area == 0f64 {
            let x = (a.x - head.x).abs() - (b.x - head.x).abs();
            let y = (a.y - head.y).abs() - (b.y - head.y).abs();

            if x < 0f64 || y < 0f64 {
                return Ordering::Less;
            } else if x > 0f64 || y > 0f64 {
                return Ordering::Greater;
            } else {
                return Ordering::Equal;
            }
        } else if area > 0f64 {
            return Ordering::Less;
        }
        return Ordering::Greater;
    });
}

fn find_lowest_point(p: &mut Vec<Point2D>) {
    let mut lowest = 0;
    for i in 1..p.len() {
        //If lowest points are on the same line, take the rightmost point
        if (p[i].y < p[lowest].y) || ((p[i].y == p[lowest].y) && p[i].x > p[lowest].x) {
            lowest = i;
        }
    }
    p.swap(0, lowest);
}

#[cfg(test)]
mod test {
    use super::convex_hull;
    use crate::structures::Point2D;
    #[test]
    fn test_convex_hull() {
        let mut points: Vec<Point2D> = Vec::new();
        // These points form a triangle, so only the 3 vertices should be in the convex hull.
        for i in 1..10 {
            points.push(Point2D::new(i as f64, i as f64));
            points.push(Point2D::new(i as f64, (-i) as f64));
            points.push(Point2D::new(i as f64, 0.0));
        }
        points.push(Point2D::new(0.0, 0.0));
        let hull = convex_hull(&mut points);
        let hull_should_be = vec![
            Point2D::new(9.0, -9.0),
            Point2D::new(9.0, 9.0),
            Point2D::new(0.0, 0.0),
        ];
        assert_eq!(hull, hull_should_be);
    }
}
