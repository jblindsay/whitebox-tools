/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/08/2018
Last Modified: 30/08/2018
License: MIT
*/

use super::do_polylines_intersect;
use crate::structures::{Point2D, Polyline};
use std::f64::EPSILON;

/// Tests if a point is Left|On|Right of an infinite line,
/// based on http://geomalgorithms.com/a03-_inclusion.html.
///
/// Input:  three points p0, p1, and p2
///
/// Return: > 0 for p2 left of the line through p0 and p1
///         = 0 for p2 on the line through p0 and p1
///         < 0 for p2 right of the line through p0 and p1
fn is_left(p0: &Point2D, p1: &Point2D, p2: &Point2D) -> f64 {
    (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y)
}

/// Tests whether a point is within in a polygon using the winding number (wn).
/// The point falls within the test polygon if the winding number is non-zero.
/// Notice that points on the edge of the poly will be deemed outside.
/// Input:   p = a point,
///          poly[] = vertex points of a polygon v[n+1] with v[n]=v[0]
pub fn point_in_poly(p: &Point2D, poly: &[Point2D]) -> bool {
    // winding_number(&p, &poly) != 0i32
    winding_number(&p, &poly) % 2 != 0i32
}

/// Calculates the Winding number (wn) test for a point in a polygon operation
/// in order to determine whether the point is within the polygon. The
/// point falls within the test polygon if the winding number is non-zero.
///
/// Input:   p = a point,
///          poly[] = vertex points of a polygon poly[n+1] with poly[n]=poly[0]
pub fn winding_number(p: &Point2D, poly: &[Point2D]) -> i32 {
    if !poly[0].nearly_equals(&poly[poly.len() - 1]) {
        panic!(
            "Error (from poly_ops::winding_num): point sequence does not form a closed polygon."
        );
    }
    let mut wn = 0i32;
    // loop through all edges of the polygon
    for i in 0..poly.len() - 1 {
        // edge from poly[i] to poly[i+1]
        if poly[i].y <= p.y {
            // start y <= p.y
            if poly[i + 1].y > p.y {
                // an upward crossing
                if is_left(&poly[i], &poly[i + 1], &p) > 0f64 {
                    // p left of edge
                    wn += 1i32; // have a valid up intersect
                }
            }
        } else {
            // start y > p.y (no test needed)
            if poly[i + 1].y <= p.y {
                // a downward crossing
                if is_left(&poly[i], &poly[i + 1], &p) < 0f64 {
                    // p right of edge
                    wn -= 1i32; // have a valid down intersect
                }
            }
        }
    }
    wn
}

/// Tests whether one polygon is contained within another polygon. Notice that
/// for polygons that are not contained within the test poly, failure occurs
/// very quickly. In the case of disjoint (non-overlapping) polys, the function
/// returns from the first tested vertex.
pub fn poly_in_poly(contained_poly: &[Point2D], containing_poly: &[Point2D]) -> bool {
    if !point_in_poly(&interior_point(contained_poly), containing_poly) {
        return false;
    }
    for p in contained_poly {
        if !point_in_poly(p, containing_poly) {
            return false;
        }
    }
    true
}

/// Tests whether one polygon overlaps another polygon. Notice that
/// for polygons that are overlap, confirmation of overlap occurs
/// very quickly. In the case of overlapping polys, the function
/// returns from the first tested vertex contained within the other.
pub fn poly_overlaps_poly(poly1: &[Point2D], poly2: &[Point2D]) -> bool {
    for p in poly1 {
        if point_in_poly(p, poly2) {
            return true;
        }
    }
    if poly_in_poly(poly1, poly2) || poly_in_poly(poly2, poly1) {
        return true;
    }
    if point_in_poly(&interior_point(poly1), poly2) {
        return true;
    }
    if point_in_poly(&interior_point(poly2), poly1) {
        return true;
    }
    if do_polylines_intersect(&Polyline::new(poly1, 0), &Polyline::new(poly2, 0)) {
        return true;
    }
    false
}

/// Return true if the polygon is convex.
pub fn poly_is_convex(poly: &[Point2D]) -> bool {
    // For each set of three adjacent points A, B, C,
    // find the cross product AB · BC. If the sign of
    // all the cross products is the same, the angles
    // are all positive or negative (depending on the
    // order in which we visit them) so the polygon
    // is convex.
    let mut got_negative = false;
    let mut got_positive = false;
    let num_points = poly.len();
    let (mut b, mut c): (usize, usize);
    let mut cross_product: f64;
    for a in 0..num_points {
        b = (a + 1) % num_points;
        c = (b + 1) % num_points;

        cross_product = (poly[a].x - poly[b].x) * (poly[c].y - poly[b].y)
            - (poly[a].y - poly[b].y) * (poly[c].x - poly[b].x);
        if cross_product < 0f64 {
            got_negative = true;
        } else if cross_product > 0f64 {
            got_positive = true;
        }
        if got_negative && got_positive {
            return false;
        }
    }

    // If we got this far, the polygon is convex.
    return true;
}

/// Returns a point that is guaranteed to be within the poly.
pub fn interior_point(poly: &[Point2D]) -> Point2D {
    if !poly[0].nearly_equals(&poly[poly.len() - 1]) {
        panic!(
            "Error (from poly_ops::interior_point): point sequence does not form a closed polygon."
        );
    }
    let num_points = poly.len();
    if num_points > 4 {
        for a in 1..num_points - 1 {
            if poly[a].is_left(&poly[a - 1], &poly[a + 1]).abs() > EPSILON {
                // it's not co-linear
                let midpoint = Point2D::midpoint(&poly[a - 1], &poly[a + 1]);
                if point_in_poly(&midpoint, &poly) {
                    return midpoint;
                }
            }
        }
        // none of the tested points were interior, return one of the edge points
        return poly[0].clone();
    } else if num_points == 4 {
        // it's a triangle
        let midpoint = Point2D::centre_point(&poly[0..3]); // The fourth point is a duplicate of the first.
        if point_in_poly(&midpoint, &poly) {
            return midpoint;
        }
        return poly[0].clone();
    }
    // you need at least four points to create a polygon (including the duplicated first/last vertex)
    panic!("Error (from poly_ops::interior_point): Could not locate polygon interior point; with only {} vertices, the feature is possibly co-linear {:?}", num_points, poly);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::structures::Point2D;
    #[test]
    fn test_point_in_poly() {
        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 0.0),
        ];
        // point inside rectangle
        assert!(point_in_poly(&Point2D::new(2.0, 2.0), &poly));
        // point outside rectangle
        assert_eq!(point_in_poly(&Point2D::new(12.0, 12.0), &poly), false);
    }

    #[test]
    fn test_winding_number() {
        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 0.0),
        ];
        // point on rectangle
        assert_eq!(winding_number(&Point2D::new(5.0, 2.0), &poly), 0i32);
        assert_eq!(winding_number(&Point2D::new(4.0, 2.0), &poly), 1i32);
        assert_eq!(winding_number(&Point2D::new(6.0, 2.0), &poly), 0i32);
    }

    #[test]
    fn test_poly_in_poly() {
        let poly1 = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 0.0),
        ];

        let poly2 = [
            Point2D::new(-1.0, -1.0),
            Point2D::new(6.0, -1.0),
            Point2D::new(6.0, 6.0),
            Point2D::new(-1.0, -1.0),
        ];

        assert!(poly_in_poly(&poly1, &poly2));

        assert_eq!(poly_in_poly(&poly2, &poly1), false);
    }

    #[test]
    fn test_poly_is_convex() {
        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 5.0),
            Point2D::new(0.0, 0.0),
        ];
        // point on rectangle
        assert!(poly_is_convex(&poly));

        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(2.5, 3.0),
            Point2D::new(0.0, 5.0),
            Point2D::new(0.0, 0.0),
        ];
        // point on rectangle
        assert_eq!(poly_is_convex(&poly), false);
    }
}
