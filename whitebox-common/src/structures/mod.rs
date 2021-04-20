// private sub-module defined in other files
mod array2d;
mod bounding_box;
mod circle;
mod fixed_radius_search;
mod line_segment;
mod n_maximizer;
mod n_minimizer;
mod point2d;
mod point3d;
mod polyline;
mod polynomial_regression_2d;
mod radial_basis_function;
mod rectangle_with_data;

// exports identifiers from private sub-modules in the current module namespace
pub use self::array2d::Array2D;
pub use self::bounding_box::BoundingBox;
pub use self::circle::Circle;
pub use self::fixed_radius_search::{DistanceMetric, FixedRadiusSearch2D, FixedRadiusSearch3D};
pub use self::line_segment::LineSegment;
pub use self::n_maximizer::NMaximizer;
pub use self::n_minimizer::NMinimizer;
pub use self::point2d::Direction;
pub use self::point2d::Point2D;
pub use self::point3d::Point3D;
pub use self::polyline::MultiPolyline;
pub use self::polyline::Polyline;
pub use self::polynomial_regression_2d::PolynomialRegression2D;
pub use self::radial_basis_function::{Basis, RadialBasisFunction};
pub use self::rectangle_with_data::RectangleWithData;
