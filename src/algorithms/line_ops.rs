/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/10/2018
Last Modified: 15/10/2018
License: MIT
*/

use crate::structures::{BoundingBox, LineSegment, Point2D, Polyline};

// pub fn lines_are_equal(line1: &[Point2D], line2: &[Point2D]) -> bool {
//     if line1.len() == line2.len() {
//         let (reverse, early_return) = if line1[0].x == line2[0].x && line1[0].y == line2[0].y {
//             (false, false)
//         } else if line1[0].x == line2[line2.len() - 1].x && line1[0].y == line2[line2.len() - 1].y {
//             (true, false)
//         } else {
//             (false, true)
//         };
//         if early_return {
//             return false;
//         }
//         // if !reverse {
//         //     for p in 0..line1.len() {
//         //         if !(line1[p].nearly_equals(&line2[p])) {
//         //             return false;
//         //         }
//         //     }
//         //     return true;
//         // } else {
//         //     for p in 0..line1.len() {
//         //         if !(line1[p].nearly_equals(&line2[line2.len() - 1 - p])) {
//         //             return false;
//         //         }
//         //     }
//         //     return true;
//         // }
//         return false;
//     }
//     false
// }

/// Perpendicular distance from a point to a line
pub fn point_line_distance(point: &Point2D, start: &Point2D, end: &Point2D) -> f64 {
    if start == end {
        return point.distance(&start);
    } else {
        let numerator = ((end.x - start.x) * (start.y - point.y)
            - (start.x - point.x) * (end.y - start.y))
            .abs();
        let denominator = start.distance(&end);
        numerator / denominator
    }
}

/// An implementation of the Ramer–Douglas–Peucker line-simplification algorithm.
/// Based on the RDP crate.
///
/// References:
/// Douglas, D.H., Peucker, T.K., 1973. Algorithms for the reduction of the number of points required to
/// represent a digitized line or its caricature. Cartographica: The International Journal for Geographic
/// Information and Geovisualization 10, 112–122. DOI
///
/// Ramer, U., 1972. An iterative procedure for the polygonal approximation of plane curves. Computer
/// Graphics and Image Processing 1, 244–256. DOI
pub fn simplify_rdp(points: &[Point2D], epsilon: &f64) -> Vec<Point2D> {
    if points.is_empty() || points.len() == 1 {
        return points.to_vec();
    }
    let mut dmax = 0.0;
    let mut index: usize = 0;
    let mut distance: f64;
    for (i, _) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        distance = point_line_distance(
            &points[i],
            &*points.first().unwrap(),
            &*points.last().unwrap(),
        );
        if distance > dmax {
            index = i;
            dmax = distance;
        }
    }
    if dmax > *epsilon {
        let mut intermediate = simplify_rdp(&points[..index + 1], &*epsilon);
        intermediate.pop();
        // recur!
        intermediate.extend_from_slice(&simplify_rdp(&points[index..], &*epsilon));
        intermediate
    } else {
        vec![*points.first().unwrap(), *points.last().unwrap()]
    }
}

pub fn find_line_intersections(line1: &[Point2D], line2: &[Point2D]) -> Vec<LineSegment> {
    let mut ret: Vec<LineSegment> = vec![];
    let box1 = BoundingBox::from_points(&line1);
    let box2 = BoundingBox::from_points(&line2);
    if box1.overlaps(box2) {
        let mut ls1: LineSegment;
        let mut ls2: LineSegment;
        for a in 0..line1.len() - 1 {
            ls1 = LineSegment::new(line1[a], line1[a + 1]);
            for b in 0..line2.len() - 1 {
                ls2 = LineSegment::new(line2[b], line2[b + 1]);
                match ls1.get_intersection(&ls2) {
                    Some(p) => ret.push(p),
                    None => {} // do nothing, the don't intersect
                }
            }
        }
    }
    ret
}

pub fn do_polylines_intersect(line1: &Polyline, line2: &Polyline) -> bool {
    let box1 = line1.get_bounding_box();
    let box2 = line2.get_bounding_box();
    if box1.overlaps(box2) {
        let mut ls1: LineSegment;
        let mut ls2: LineSegment;
        for a in 0..line1.len() - 1 {
            ls1 = LineSegment::new(line1[a], line1[a + 1]);
            for b in 0..line2.len() - 1 {
                ls2 = LineSegment::new(line2[b], line2[b + 1]);
                match ls1.get_intersection(&ls2) {
                    Some(_) => {
                        return true;
                    }
                    None => {} // do nothing, the don't intersect
                }
            }
        }
    }
    false
}

pub fn find_split_points_at_line_intersections(line1: &mut Polyline, line2: &mut Polyline) {
    let box1 = line1.get_bounding_box();
    let box2 = line2.get_bounding_box();
    if box1.overlaps(box2) {
        let mut ls1: LineSegment;
        let mut ls2: LineSegment;
        for a in 0..line1.len() - 1 {
            ls1 = LineSegment::new(line1[a], line1[a + 1]);
            for b in 0..line2.len() - 1 {
                ls2 = LineSegment::new(line2[b], line2[b + 1]);
                match ls1.get_intersection(&ls2) {
                    Some(ls) => {
                        line1.insert_split_point(
                            a as f64
                                + ls.p1.distance_squared(&ls1.p1)
                                    / ls1.p2.distance_squared(&ls1.p1), //(ls.p1.x - ls1.p1.x) / (ls1.p2.x - ls1.p1.x),
                            ls.p1,
                        );
                        line2.insert_split_point(
                            b as f64
                                + ls.p1.distance_squared(&ls2.p1)
                                    / ls2.p2.distance_squared(&ls2.p1), //(ls.p1.x - ls2.p1.x) / (ls2.p2.x - ls2.p1.x),
                            ls.p1,
                        );
                        if ls.p1 != ls.p2 {
                            line1.insert_split_point(
                                a as f64
                                    + ls.p2.distance_squared(&ls1.p1)
                                        / ls1.p2.distance_squared(&ls1.p1), //(ls.p2.x - ls1.p1.x) / (ls1.p2.x - ls1.p1.x),
                                ls.p2,
                            );
                            line2.insert_split_point(
                                b as f64
                                    + ls.p2.distance_squared(&ls2.p1)
                                        / ls2.p2.distance_squared(&ls2.p1), //(ls.p2.x - ls2.p1.x) / (ls2.p2.x - ls2.p1.x),
                                ls.p2,
                            );
                        }
                    }
                    None => {} // do nothing, the don't intersect
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::find_line_intersections;
    use crate::structures::{LineSegment, Point2D};

    #[test]
    fn test_find_line_intersections() {
        let line1 = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(10.0, 10.0),
            Point2D::new(12.0, 6.0),
            Point2D::new(6.0, 0.0),
        ];
        let line2 = vec![
            Point2D::new(-1.0, 5.0),
            Point2D::new(6.0, 5.0),
            Point2D::new(6.0, 2.0),
            Point2D::new(12.0, 2.0),
        ];

        let intersections = find_line_intersections(&line1, &line2);
        let intersections_should_be = vec![
            LineSegment::new(Point2D::new(5.0, 5.0), Point2D::new(5.0, 5.0)),
            LineSegment::new(Point2D::new(8.0, 2.0), Point2D::new(8.0, 2.0)),
        ];
        assert_eq!(intersections, intersections_should_be);
    }

    #[test]
    fn test_no_lines_intersections() {
        let line1 = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(10.0, 10.0),
            Point2D::new(12.0, 6.0),
            Point2D::new(6.0, 0.0),
        ];
        let line2 = vec![Point2D::new(-1.0, -5.0), Point2D::new(-6.0, -5.0)];

        let intersections = find_line_intersections(&line1, &line2);
        assert_eq!(intersections.len(), 0);
    }

    #[test]
    fn test_vertical_lines_intersections() {
        let line1 = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 10.0)];
        let line2 = vec![Point2D::new(5.0, 1.0), Point2D::new(5.0, 10.0)];

        let intersections = find_line_intersections(&line1, &line2);
        let intersections_should_be = vec![LineSegment::new(
            Point2D::new(5.0, 5.0),
            Point2D::new(5.0, 5.0),
        )];
        assert_eq!(intersections, intersections_should_be);
    }

    #[test]
    fn test_coincident_lines_intersections() {
        let line1 = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 10.0)];
        let line2 = vec![Point2D::new(5.0, 5.0), Point2D::new(18.0, 18.0)];

        let intersections = find_line_intersections(&line1, &line2);
        let intersections_should_be = vec![LineSegment::new(
            Point2D::new(5.0, 5.0),
            Point2D::new(10.0, 10.0),
        )];
        assert_eq!(intersections, intersections_should_be);
    }
}
