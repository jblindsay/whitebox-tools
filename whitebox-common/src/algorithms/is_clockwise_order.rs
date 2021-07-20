use crate::structures::Point2D;

/// Checks whether a sequence of Point2D are in clockwise order.
pub fn is_clockwise_order(points: &[Point2D]) -> bool {
    // This approach is based on the method described by Paul Bourke, March 1998
    // http://paulbourke.net/geometry/clockwise/index.html

    let mut n1: usize;
    let mut n2: usize;
    let mut n3: usize;

    let st_point = 0usize;

    let end_point = if points[0] == points[points.len() - 1] {
        // The last point is the same as the first...it's not a legitemate point.
        points.len() - 2
    } else {
        points.len() - 1
    };

    let num_points_in_part = end_point - st_point + 1;

    if num_points_in_part < 3 {
        return false;
    } // something's wrong!

    // first see if it is a convex or concave polygon
    // calculate the cross product for each adjacent edge.
    let mut crossproducts = vec![0f64; num_points_in_part];
    for j in 0..num_points_in_part {
        n2 = st_point + j;
        if j == 0 {
            n1 = st_point + num_points_in_part - 1;
            n3 = st_point + j + 1;
        } else if j == num_points_in_part - 1 {
            n1 = st_point + j - 1;
            n3 = st_point;
        } else {
            n1 = st_point + j - 1;
            n3 = st_point + j + 1;
        }
        crossproducts[j] = (points[n2].x - points[n1].x) * (points[n3].y - points[n2].y)
            - (points[n2].y - points[n1].y) * (points[n3].x - points[n2].x);
    }

    let test_sign = crossproducts[0] >= 0f64;
    let mut is_convex = true;
    for j in 1..num_points_in_part {
        if crossproducts[j] >= 0f64 && !test_sign {
            is_convex = false;
            break;
        } else if crossproducts[j] < 0f64 && test_sign {
            is_convex = false;
            break;
        }
    }

    // now see if it is clockwise or counter-clockwise
    if is_convex {
        if test_sign {
            // positive means counter-clockwise
            return false;
        } else {
            return true;
        }
    } else {
        // calculate the polygon area. If it's positive it's in clockwise order, else counter-clockwise.
        let mut area = 0f64;
        for j in 0..num_points_in_part {
            n1 = st_point + j;
            if j < num_points_in_part - 1 {
                n2 = st_point + j + 1;
            } else {
                n2 = st_point;
            }

            area += (points[n1].x * points[n2].y) - (points[n2].x * points[n1].y);
        }
        area /= 2.0;

        if area < 0f64 {
            // a positive area indicates counter-clockwise order
            return true;
        } else {
            return false;
        }
    }
}

#[cfg(test)]
mod test {
    use super::is_clockwise_order;
    use crate::structures::Point2D;
    #[test]
    fn test_is_clockwise_order() {
        let mut points: Vec<Point2D> = Vec::new();
        points.push(Point2D::new(0f64, 0f64));
        points.push(Point2D::new(1f64, 0f64));
        points.push(Point2D::new(1f64, 1f64));
        points.push(Point2D::new(0f64, 1f64));
        points.push(Point2D::new(0f64, 0f64));

        assert_eq!(is_clockwise_order(&points), false);

        points.reverse();
        assert_eq!(is_clockwise_order(&points), true);
    }
}
