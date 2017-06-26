// private sub-module defined in other files
mod mean_filter;
mod total_filter;

// exports identifiers from private sub-modules in the current module namespace
pub use self::mean_filter::MeanFilter;
pub use self::total_filter::TotalFilter;