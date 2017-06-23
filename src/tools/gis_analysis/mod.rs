// private sub-module defined in other files
mod average_overlay;
mod buffer_raster;
mod clump;
mod euclidean_allocation;
mod euclidean_distance;
mod highest_pos;
mod lowest_pos;
mod max_abs_overlay;
mod max_overlay;
mod min_abs_overlay;
mod min_overlay;
mod quantiles;

// exports identifiers from private sub-modules in the current module namespace
pub use self::average_overlay::AverageOverlay;
pub use self::buffer_raster::BufferRaster;
pub use self::clump::Clump;
pub use self::euclidean_allocation::EuclideanAllocation;
pub use self::euclidean_distance::EuclideanDistance;
pub use self::highest_pos::HighestPosition;
pub use self::lowest_pos::LowestPosition;
pub use self::max_abs_overlay::MaxAbsoluteOverlay;
pub use self::max_overlay::MaxOverlay;
pub use self::min_abs_overlay::MinAbsoluteOverlay;
pub use self::min_overlay::MinOverlay;
pub use self::quantiles::Quantiles;
