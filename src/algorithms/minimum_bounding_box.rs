/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/09/2018
Last Modified: 04/09/2018
License: MIT
*/

use algorithms::convex_hull;
use std::f64;
use structures::Point2D;

/// Returns the minimum bounding box (MBB) around a set of points (Vec<Point2D>.
/// The algorithm first calculates the convex hull around the points. The MBB
/// will be aligned with one of the sides of the convex hull. If minimize_area
/// is set to true, the bounding box will be selected to minimize box area;
/// otherwise, it will minimize box perimeter.
pub fn minimum_bounding_box(points: &mut Vec<Point2D>) -> Vec<Point2D> {
    // Get the convex hull
    let hull = convex_hull(points);
    let num_hull_pts = hull.len();

    println!("Raw Points...");
    for p in points {
        println!("{}, {}", p.x, p.y);
    }

    println!("Hull Points...");
    for p in &hull {
        println!("{}, {}", p.x, p.y);
    }

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

    // let mut vertices_rotated = vec![Point2D { x: 0f64, y: 0f64 }; num_hull_pts];

    let mut x_axis = 9999999f64;
    let mut y_axis = 9999999f64;
    let mut slope = 0f64;
    let mut box_centre_x = 0f64;
    let mut box_centre_y = 0f64;
    let (mut x, mut y): (f64, f64);
    let (mut x_rotated, mut y_rotated): (f64, f64);
    let (mut new_x_axis, mut new_y_axis): (f64, f64);
    let mut current_area: f64;
    let mut min_area = f64::INFINITY;
    let right_angle = f64::consts::PI / 2f64;

    // Rotate the hull points to align with the orientation of each side in order.
    for m in 0..num_hull_pts - 1 {
        let psi = -((hull[m + 1].x - hull[m].x).atan2(hull[m + 1].y - hull[m].y));

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

        new_x_axis = east - west;
        new_y_axis = north - south;
        current_area = new_x_axis * new_y_axis;
        if current_area < min_area {
            min_area = current_area;
            x_axis = new_x_axis;
            y_axis = new_y_axis;
            slope = if x_axis < y_axis {
                -psi
            } else {
                -(right_angle + psi)
            };
            x = west + x_axis / 2f64;
            y = north + y_axis / 2f64;
            box_centre_x = midx + (x * (-psi).cos()) - (y * (-psi).sin());
            box_centre_y = midy + (x * (-psi).sin()) + (y * (-psi).cos());
        }
    }
    let long_axis = x_axis.max(y_axis);
    let short_axis = x_axis.min(y_axis);

    // double[][] axesEndPoints = new double[4][2];
    // axesEndPoints[0][0] = box_centre_x + long_axis / 2.0 * slope.cos();
    // axesEndPoints[0][1] = box_centre_y + long_axis / 2.0 * slope.sin();
    // axesEndPoints[1][0] = box_centre_x - long_axis / 2.0 * slope.cos();
    // axesEndPoints[1][1] = box_centre_y - long_axis / 2.0 * slope.sin();
    // axesEndPoints[2][0] = box_centre_x + short_axis / 2.0 * (right_angle + slope).cos();
    // axesEndPoints[2][1] = box_centre_y + short_axis / 2.0 * (right_angle + slope).sin();
    // axesEndPoints[3][0] = box_centre_x - short_axis / 2.0 * (right_angle + slope).cos();
    // axesEndPoints[3][1] = box_centre_y - short_axis / 2.0 * (right_angle + slope).sin();

    let mut ret: Vec<Point2D> = Vec::with_capacity(4);

    ret.push(Point2D {
        x: box_centre_x
            + long_axis / 2.0 * slope.cos()
            + short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y
            + long_axis / 2.0 * slope.sin()
            + short_axis / 2.0 * (right_angle + slope).sin(),
    });

    ret.push(Point2D {
        x: box_centre_x + long_axis / 2.0 * slope.cos()
            - short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y + long_axis / 2.0 * slope.sin()
            - short_axis / 2.0 * (right_angle + slope).sin(),
    });

    ret.push(Point2D {
        x: box_centre_x
            - long_axis / 2.0 * slope.cos()
            - short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y
            - long_axis / 2.0 * slope.sin()
            - short_axis / 2.0 * (right_angle + slope).sin(),
    });

    ret.push(Point2D {
        x: box_centre_x - long_axis / 2.0 * slope.cos()
            + short_axis / 2.0 * (right_angle + slope).cos(),
        y: box_centre_y - long_axis / 2.0 * slope.sin()
            + short_axis / 2.0 * (right_angle + slope).sin(),
    });

    println!("Hull Points...");
    for p in &ret {
        println!("{}, {}", p.x, p.y);
    }

    ret
}

#[cfg(test)]
mod test {
    use super::minimum_bounding_box;
    use structures::Point2D;
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

        let mbb = minimum_bounding_box(&mut points);

        let mbb_should_be = vec![
            Point2D::new(-10.0, 10.0),
            Point2D::new(10.0, 10.0),
            Point2D::new(-10.0, -10.0),
            Point2D::new(10.0, -10.0),
        ];
        assert_eq!(mbb, mbb_should_be);
    }

}
