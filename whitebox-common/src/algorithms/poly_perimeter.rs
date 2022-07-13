/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/09/2018
Last Modified: 25/09/2018
License: MIT
*/

use crate::structures::Point2D;

/// Calculates the perimeter of a polygon defined by a series of vertices.
pub fn polygon_perimeter(vertices: &[Point2D]) -> f64 {
    let num_vertices = vertices.len();

    let mut perimeter = 0f64;

    for i in 0..num_vertices - 1 {
        perimeter += vertices[i].distance(&vertices[i + 1]);
    }

    perimeter += vertices[num_vertices - 1].distance(&vertices[0]);

    perimeter
}

#[cfg(test)]
mod test {
    use super::polygon_perimeter;
    use crate::structures::Point2D;
    #[test]
    fn test_closed_polygon_perimeter() {
        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 5.0),
            Point2D::new(0.0, 0.0),
        ];
        assert_eq!(polygon_perimeter(&poly), 20f64);
    }

    #[test]
    fn test_open_polygon_perimeter() {
        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 5.0),
        ];
        assert_eq!(polygon_perimeter(&poly), 20f64);
    }
}
