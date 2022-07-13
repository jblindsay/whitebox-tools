/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/09/2018
Last Modified: 25/09/2018
License: MIT
*/

use crate::structures::Point2D;

/// Calculates the area of a polygon defined by a series of vertices.
pub fn polygon_area(vertices: &[Point2D]) -> f64 {
    let num_vertices = vertices.len();

    let mut area = 0f64;

    for i in 0..num_vertices - 1 {
        area += vertices[i].x * vertices[i + 1].y - vertices[i + 1].x * vertices[i].y;
    }

    area +=
        vertices[num_vertices - 1].x * vertices[0].y - vertices[0].x * vertices[num_vertices - 1].y;

    area.abs() / 2.0f64
}

#[cfg(test)]
mod test {
    use super::polygon_area;
    use crate::structures::Point2D;
    #[test]
    fn test_closed_polygon_area() {
        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 5.0),
            Point2D::new(0.0, 0.0),
        ];
        assert_eq!(polygon_area(&poly), 25f64);
    }

    #[test]
    fn test_open_polygon_area() {
        let poly = [
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 5.0),
        ];
        assert_eq!(polygon_area(&poly), 25f64);
    }
}
