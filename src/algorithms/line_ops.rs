/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/10/2018
Last Modified: 15/10/2018
License: MIT
*/

use structures::{BoundingBox, LineSegment, Point2D, Polyline};

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
    use structures::{LineSegment, Point2D};

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
