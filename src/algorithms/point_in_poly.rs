/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/08/2018
Last Modified: 30/08/2018
License: MIT
*/

pub use structures::Point2D;

/// Tests if a point is Left|On|Right of an infinite line,
/// based on http://geomalgorithms.com/a03-_inclusion.html.
///
/// Input:  three points P0, P1, and P2
///
/// Return: >0 for P2 left of the line through P0 and P1
///         =0 for P2  on the line
///         <0 for P2  right of the line
fn is_left(p0: &Point2D, p1: &Point2D, p2: &Point2D) -> f64 {
    (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y)
}

/// Calculates the Winding number test for a point in a polygon operation.
///
/// Input:   p = a point,
///          v[] = vertex points of a polygon v[n+1] with v[n]=v[0]
///
/// Return:  wn = the winding number (=0 only when p is outside)
pub fn point_in_poly(p: &Point2D, v: &[Point2D]) -> bool {
    if v[0] != v[v.len() - 1] {
        panic!("Warning, point squence do not form a closed polygon.");
    }
    let mut wn = 0i32;
    // loop through all edges of the polygon
    for i in 0..v.len() - 1 {
        // edge from v[i] to v[i+1]
        if v[i].y <= p.y {
            // start y <= p.y
            if v[i + 1].y > p.y {
                // an upward crossing
                if is_left(&v[i], &v[i + 1], &p) > 0f64 {
                    // p left of edge
                    wn += 1i32; // have a valid up intersect
                }
            }
        } else {
            // start y > p.y (no test needed)
            if v[i + 1].y <= p.y {
                // a downward crossing
                if is_left(&v[i], &v[i + 1], &p) < 0f64 {
                    // p right of edge
                    wn -= 1i32; // have a valid down intersect
                }
            }
        }
    }
    wn != 0i32
}

#[cfg(test)]
mod test {
    use super::point_in_poly;
    use structures::Point2D;
    #[test]
    fn test_point_in_poly() {
        let poly = vec![
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

}
