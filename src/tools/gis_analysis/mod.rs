// private sub-module defined in other files
mod euclidean_allocation;
mod euclidean_distance;

// exports identifiers from private sub-modules in the current module namespace
pub use self::euclidean_allocation::EuclideanAllocation;
pub use self::euclidean_distance::EuclideanDistance;
