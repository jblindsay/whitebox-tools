/* 
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: July 17, 2017
License: MIT
*/

/* 
Eventually this will be used to support multiple vector formats but
for now it's just Shapefiles.
*/

// private sub-module defined in other files
mod shapefile;

// exports identifiers from private sub-modules in the current module namespace
pub use self::shapefile::Shapefile;