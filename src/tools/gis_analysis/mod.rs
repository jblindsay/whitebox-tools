// private sub-module defined in other files
mod average_overlay;
mod buffer_raster;
mod clump;
mod euclidean_allocation;
mod euclidean_distance;
mod quantiles;

// exports identifiers from private sub-modules in the current module namespace
pub use self::average_overlay::AverageOverlay;
pub use self::buffer_raster::BufferRaster;
pub use self::clump::Clump;
pub use self::euclidean_allocation::EuclideanAllocation;
pub use self::euclidean_distance::EuclideanDistance;
pub use self::quantiles::Quantiles;
