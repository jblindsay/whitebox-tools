/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/08/2018
Last Modified: 25/09/2018
License: MIT
*/
// private sub-module defined in other files
mod convex_hull;
mod delaunay_triangulation;
mod is_clockwise_order;
mod line_ops;
// mod lzw;
mod minimum_bounding_box;
mod poly_area;
mod poly_ops;
mod poly_perimeter;
mod smallest_enclosing_circle;

// exports identifiers from private sub-modules in the current module namespace
pub use self::convex_hull::convex_hull;
pub use self::delaunay_triangulation::{triangulate, Triangulation};
pub use self::delaunay_triangulation::EMPTY;
pub use self::is_clockwise_order::is_clockwise_order;
pub use self::line_ops::{
    do_polylines_intersect, find_line_intersections, find_split_points_at_line_intersections,
    point_line_distance, simplify_rdp,
};
// pub use self::lzw::{lzw_decode, lzw_encode};
pub use self::minimum_bounding_box::{minimum_bounding_box, MinimizationCriterion};
pub use self::poly_area::polygon_area;
pub use self::poly_ops::{
    interior_point, point_in_poly, poly_in_poly, poly_is_convex, poly_overlaps_poly, winding_number,
};
pub use self::poly_perimeter::polygon_perimeter;
pub use self::smallest_enclosing_circle::smallest_enclosing_circle;
