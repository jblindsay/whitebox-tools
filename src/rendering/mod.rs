// private sub-module defined in other files
mod histogram;


// exports identifiers from private sub-modules in the current module namespace
pub use self::histogram::Histogram;
pub mod html;