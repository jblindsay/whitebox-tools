use crate::rstar::primitives::Rectangle;
use crate::rstar::{Envelope, Point, PointDistance, RTreeObject, AABB};

type RectangleF64 = Rectangle<[f64; 2]>;

#[derive(Debug)]
pub struct RectangleWithData<T> {
    pub data: T,
    pub rectangle: RectangleF64,
}

impl<T> RectangleWithData<T> {
    pub fn new(data: T, corner1: [f64; 2], corner2: [f64; 2]) -> Self {
        let rectangle = Rectangle::from_corners(corner1, corner2);
        RectangleWithData { data, rectangle }
    }
}

impl<T> RTreeObject for RectangleWithData<T> {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        // AABB::from_point([self.x_coordinate, self.y_coordinate])
        self.rectangle.envelope()
    }
}

impl<T> RectangleWithData<T> {
    /// Returns the nearest point within this rectangle to a given point.
    ///
    /// If `query_point` is contained within this rectangle, `query_point` is returned.
    pub fn nearest_point(&self, query_point: &[f64; 2]) -> [f64; 2] {
        self.rectangle.nearest_point(query_point)
    }
}

impl<T> PointDistance for RectangleWithData<T> {
    fn distance_2(
        &self,
        point: &<Self::Envelope as Envelope>::Point,
    ) -> <<Self::Envelope as Envelope>::Point as Point>::Scalar {
        self.rectangle.distance_2(point)
    }

    fn contains_point(&self, point: &<Self::Envelope as Envelope>::Point) -> bool {
        self.rectangle.contains_point(point)
    }

    fn distance_2_if_less_or_equal(
        &self,
        point: &<Self::Envelope as Envelope>::Point,
        max_distance_2: <<Self::Envelope as Envelope>::Point as Point>::Scalar,
    ) -> Option<<<Self::Envelope as Envelope>::Point as Point>::Scalar> {
        let distance_2 = self.distance_2(point);
        if distance_2 <= max_distance_2 {
            Some(distance_2)
        } else {
            None
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::RectangleWithData;
//     use crate::rstar::{PointDistance, RTree};

//     #[test]
//     fn rectangle_distance() {
//         let rectangle = RectangleWithData::new(1, [0.5, 0.5], [1.0, 2.0]);
//         let small_val = 0.00001;
//         assert!((rectangle.distance_2(&[0.5, 0.5]) - 0.0) < small_val);
//         assert!((rectangle.distance_2(&[0.0, 0.5]) - 0.5 * 0.5) < small_val);
//         assert!((rectangle.distance_2(&[0.5, 1.0]) - 0.0) < small_val);
//         assert!((rectangle.distance_2(&[0.0, 0.0]) - 0.5) < small_val);
//         assert!((rectangle.distance_2(&[0.0, 1.0]) - 0.5 * 0.5) < small_val);
//         assert!((rectangle.distance_2(&[1.0, 3.0]) - 1.0) < small_val);
//         assert!((rectangle.distance_2(&[1.0, 1.0]) - 0.0) < small_val);
//     }

//     #[test]
//     fn rectangle_locate_all_at_point() {
//         let tree = RTree::bulk_load(vec![
//             RectangleWithData::new(1, [0.0, 0.0], [2.0, 2.0]),
//             RectangleWithData::new(2, [1.0, 1.0], [3.0, 3.0]),
//             RectangleWithData::new(3, [2.5, 2.5], [4.0, 4.0]),
//         ]);

//         assert_eq!(tree.locate_all_at_point(&[1.5, 1.5]).count(), 2);
//         assert_eq!(tree.locate_all_at_point(&[0.0, 0.0]).count(), 1);
//         assert_eq!(tree.locate_all_at_point(&[-1., 0.0]).count(), 0);
//         assert_eq!(tree.locate_all_at_point(&[2.6, 2.6]).count(), 2);

//         let ret = tree.locate_all_at_point(&[1.5, 1.5]).collect::<Vec<_>>();
//         assert_eq!(ret[0].data, 2);
//         assert_eq!(ret[1].data, 1);
//     }
// }
