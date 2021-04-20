/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/09/2018
Last Modified: 14/09/2018
License: MIT
*/

use super::convex_hull;
use crate::structures::Point2D;
use std::f64;

#[derive(Copy, Clone, Debug)]
pub enum MinimizationCriterion {
    Area,
    Perimeter,
    Length,
    Width,
}

/// Returns the minimum bounding box (MBB) around a set of points (Vec<Point2D>.
/// The algorithm first calculates the convex hull around the points. The MBB
/// will be aligned with one of the sides of the convex hull. If minimize_area
/// is set to true, the bounding box will be selected to minimize box area;
/// otherwise, it will minimize box perimeter.
///
/// The return is a Vec<Point2D> of the four corner points of the MBB.
pub fn minimum_bounding_box(
    points: &mut Vec<Point2D>,
    min_criterion: MinimizationCriterion,
) -> Vec<Point2D> {
    // Get the minimization criteria function
    let min_fn: Box<dyn Fn(f64, f64) -> f64> = match min_criterion {
        MinimizationCriterion::Area => Box::new(|axis1: f64, axis2: f64| -> f64 { axis1 * axis2 }),
        MinimizationCriterion::Perimeter => {
            Box::new(|axis1: f64, axis2: f64| -> f64 { 2f64 * axis1 + 2f64 * axis2 })
        }
        MinimizationCriterion::Length => {
            Box::new(|axis1: f64, axis2: f64| -> f64 { axis1.max(axis2) })
        }
        MinimizationCriterion::Width => {
            Box::new(|axis1: f64, axis2: f64| -> f64 { axis1.min(axis2) })
        }
    };

    // Get the convex hull
    let hull = convex_hull(points);
    let num_hull_pts = hull.len();

    // find the mid-point of the points
    let mut east = f64::NEG_INFINITY;
    let mut west = f64::INFINITY;
    let mut north = f64::NEG_INFINITY;
    let mut south = f64::INFINITY;

    for m in 0..num_hull_pts {
        if hull[m].x > east {
            east = hull[m].x;
        }
        if hull[m].x < west {
            west = hull[m].x;
        }
        if hull[m].y > north {
            north = hull[m].y;
        }
        if hull[m].y < south {
            south = hull[m].y;
        }
    }

    let midx = west + (east - west) / 2f64;
    let midy = south + (north - south) / 2f64;

    let mut x_axis = 9999999f64;
    let mut y_axis = 9999999f64;
    let mut slope = 0f64;
    let mut box_centre_x = 0f64;
    let mut box_centre_y = 0f64;
    let (mut x, mut y): (f64, f64);
    let (mut x_rotated, mut y_rotated): (f64, f64);
    let (mut new_x_axis, mut new_y_axis): (f64, f64);
    let mut current_metric: f64;
    let mut min_metric = f64::INFINITY;
    let right_angle = f64::consts::PI / 2f64;

    // Rotate the hull points to align with the orientation of each side in order.
    for m in 0..num_hull_pts - 1 {
        let psi = -((hull[m + 1].y - hull[m].y).atan2(hull[m + 1].x - hull[m].x));

        // rotate the hull points and find the axis-aligned bounding box
        east = f64::NEG_INFINITY;
        west = f64::INFINITY;
        north = f64::NEG_INFINITY;
        south = f64::INFINITY;
        for n in 0..num_hull_pts {
            x = hull[n].x - midx;
            y = hull[n].y - midy;
            x_rotated = x * psi.cos() - y * psi.sin();
            y_rotated = x * psi.sin() + y * psi.cos();

            if x_rotated > east {
                east = x_rotated;
            }
            if x_rotated < west {
                west = x_rotated;
            }
            if y_rotated > north {
                north = y_rotated;
            }
            if y_rotated < south {
                south = y_rotated;
            }
        }

        new_x_axis = (east - west).abs();
        new_y_axis = (north - south).abs();
        current_metric = min_fn(new_x_axis, new_y_axis);
        if current_metric < min_metric {
            min_metric = current_metric;
            x_axis = new_x_axis;
            y_axis = new_y_axis;
            slope = if x_axis > y_axis {
                -psi
            } else {
                -(right_angle + psi)
            };
            x = west + x_axis / 2f64;
            y = north - y_axis / 2f64;
            box_centre_x = midx + (x * (-psi).cos()) - (y * (-psi).sin());
            box_centre_y = midy + (x * (-psi).sin()) + (y * (-psi).cos());
        }
    }

    let long_axis = x_axis.max(y_axis);
    let short_axis = x_axis.min(y_axis);

    let mut corner_pnts: Vec<Point2D> = Vec::with_capacity(4);

    corner_pnts.push(Point2D {
        x: box_centre_x
            + long_axis / 2.0 * slope.cos()
            + short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y
            + long_axis / 2.0 * slope.sin()
            + short_axis / 2.0 * (right_angle + slope).sin(),
    });

    corner_pnts.push(Point2D {
        x: box_centre_x + long_axis / 2.0 * slope.cos()
            - short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y + long_axis / 2.0 * slope.sin()
            - short_axis / 2.0 * (right_angle + slope).sin(),
    });

    corner_pnts.push(Point2D {
        x: box_centre_x
            - long_axis / 2.0 * slope.cos()
            - short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y
            - long_axis / 2.0 * slope.sin()
            - short_axis / 2.0 * (right_angle + slope).sin(),
    });

    corner_pnts.push(Point2D {
        x: box_centre_x - long_axis / 2.0 * slope.cos()
            + short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y - long_axis / 2.0 * slope.sin()
            + short_axis / 2.0 * (right_angle + slope).sin(),
    });

    corner_pnts
}

#[cfg(test)]
mod test {
    use super::{minimum_bounding_box, MinimizationCriterion};
    use crate::structures::Point2D;
    #[test]
    fn test_minimum_bounding_box() {
        let mut points: Vec<Point2D> = Vec::new();
        points.push(Point2D::new(-10.0, 10.0));
        points.push(Point2D::new(10.0, 10.0));
        points.push(Point2D::new(-10.0, -10.0));
        points.push(Point2D::new(10.0, -10.0));
        points.push(Point2D::new(0.0, 0.0));
        points.push(Point2D::new(1.0, 1.0));
        points.push(Point2D::new(15.0, 15.0));
        points.push(Point2D::new(-15.0, -15.0));

        let mbb = minimum_bounding_box(&mut points, MinimizationCriterion::Area);

        let mbb_should_be = vec![
            Point2D::new(15f64, 15.000000000000002f64),
            Point2D::new(19.615384615384613f64, -8.076923076923078f64),
            Point2D::new(-15f64, -15.000000000000002f64),
            Point2D::new(-19.615384615384613f64, 8.076923076923078f64),
        ];
        assert_eq!(mbb, mbb_should_be);
    }
}
