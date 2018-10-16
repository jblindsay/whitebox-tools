/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/10/2018
Last Modified: 15/10/2018
License: MIT
*/

use structures::{BoundingBox, LineSegment, Point2D};

pub fn find_lines_intersections(line1: &[Point2D], line2: &[Point2D]) -> Vec<LineSegment> {
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

// pub fn insert_line_intersections(
//     line1: &[Point2D],
//     line2: &[Point2D],
// ) -> (Vec<Point2D>, Vec<Point2D>) {
//     let mut l1: Vec<Point2D> = vec![];
//     let mut l2: Vec<Point2D> = vec![];
//     let box1 = BoundingBox::from_points(&line1);
//     let box2 = BoundingBox::from_points(&line2);
//     if box1.overlaps(box2) {
//         let mut ls1: LineSegment;
//         let mut ls2: LineSegment;
//         let mut intersections1: Vec<(usize, Point2D)> = vec![];
//         for a in 0..line1.len() - 1 {
//             l1.push(line1[a]);
//             ls1 = LineSegment::new(line1[a], line1[a + 1]);
//             for b in 0..line2.len() - 1 {
//                 ls2 = LineSegment::new(line2[b], line2[b + 1]);
//                 match ls1.get_intersection(&ls2) {
//                     Some(p) => {
//                         intersections1.push((a, p.p1));
//                         if p.p1 != p.p2 {
//                             // it intersects at a line, not a point
//                             intersections1.push((a, p.p2));
//                         }
//                     }
//                     None => {} // do nothing, the don't intersect
//                 }
//             }
//         }
//     } else {
//         // there can be no intersections
//         l1 = line1.clone();
//         l2 = line2.clone();
//     }
//     (l1, l2)
// }

#[cfg(test)]
mod test {
    use super::find_lines_intersections;
    use structures::{LineSegment, Point2D};

    #[test]
    fn test_find_lines_intersections() {
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

        let intersections = find_lines_intersections(&line1, &line2);
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

        let intersections = find_lines_intersections(&line1, &line2);
        assert_eq!(intersections.len(), 0);
    }

    #[test]
    fn test_vertical_lines_intersections() {
        let line1 = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 10.0)];
        let line2 = vec![Point2D::new(5.0, 1.0), Point2D::new(5.0, 10.0)];

        let intersections = find_lines_intersections(&line1, &line2);
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

        let intersections = find_lines_intersections(&line1, &line2);
        let intersections_should_be = vec![LineSegment::new(
            Point2D::new(5.0, 5.0),
            Point2D::new(10.0, 10.0),
        )];
        assert_eq!(intersections, intersections_should_be);
    }
}
