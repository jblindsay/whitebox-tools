// private sub-module defined in other files
mod box_whisker;
mod histogram;
mod line_graph;
mod scattergram;

// exports identifiers from private sub-modules in the current module namespace
pub use self::box_whisker::BoxAndWhiskerPlot;
pub use self::histogram::Histogram;
pub use self::line_graph::LineGraph;
pub use self::scattergram::Scattergram;
pub mod html;
