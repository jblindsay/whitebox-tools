# WhiteboxTools

- [Desciption](#description)
- [Installation](#installation)
- [Usage](#usage)
- [Available Tools](#available-tools)
- [Contributing](#contributing)
- [Contributors](#contributors)
- [License](#license)
- [Known Issues](#known-issues)

## Description

**WhiteboxTools** is an experimental library, developed using the Rust programming language, for analyzing geospatial data. Although it is intended to serve as a source of plugin tools for the [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/) open-source GIS project, the tools contained in this library are stand-alone and can run outside of the larger Whitebox GAT project. See [Usage](#usage) for further details. There have been a large number of requests to call *Whitebox GAT* tools and functionality from outside of the Whitebox user-interface (e.g. from Python automation scripts). *WhiteboxTools* is intended to meet these usage requirements. Eventually most of the approximately 450 tools contained within *Whitebox GAT* will be ported to *WhiteboxTools*. In addition to separating the processing capabilities and the user-interface (and reducing the reliance on Java), this migration should significantly improve processing efficiency. This is because Rust is generally [faster than the equivalent Java code](http://benchmarksgame.alioth.debian.org/u64q/compare.php?lang=rust&lang2=java) and because many of the *WhiteboxTools* functions are designed to process in parallel wherever possible. In contrast, the older Java codebase included largely single-threaded applications. 

The *WhiteboxTools* project is related to the [**GoSpatial**](https://github.com/jblindsay/go-spatial) project, which has similar goals but is designed using the Go programming language instead of Rust. Both projects are currently considered to be experimental.

## Installation

To install the latest development version of *WhiteboxTools* Ensure that the latest stable version of the [Rust programming language](https://www.rust-lang.org) compiler is installed on your machine. Fork the GitHub repository then run the build.py Python script. The whitebox-tools.exe executable file will be located within the /target/release/ folder. Pre-compiled binaries for *WhiteboxTools* will be distributed with releases of *Whitebox GAT* for each of the supported platforms.

## Usage

For examples of how to call functions and run tools from *WhiteboxTools*, see the *whitebox_example.py* Python script, which itself uses the *whitebox_tools.py* script as an interface for interacting with the executable file. The *whitebox_tools.py* script calls the executable using subprocesses rather than as a dynamic library. Future versions may compile the library as a dynamic shared object if this is preferred.

## Available Tools

Eventually most of *Whitebox GAT'* approximately 450 tools will be ported to *WhiteboxTools*, although this is an immense task. Support for vector data (Shapefile) reading/writing and a topological analysis library will need to be added to port any of the tools involving vector spatial data. Opportunities to parallelize existing tools will be sought during porting. All new plugin tools will be added to *Whitebox GAT* using this library of functions. The library currently contains the following tools:

**GIS Analysis**
- ***BufferRaster***: Maps a distance-based buffer around each non-background (non-zero/non-nodata) grid cell in an input image.
- ***EuclideanAllocation***: Assigns grid cells in the output raster the value of the nearest target cell in the input image, measured by the Shih and Wu (2004) Euclidean distance transform.
- ***EuclideanDistance***: Calculates the Shih and Wu (2004) Euclidean distance transform.
- ***Quantiles***: Tranforms raster values into quantiles.

**Hydrological Analysis**
- ***D8Pointer***: Calculates a D8 flow pointer raster from an input DEM.

**LiDAR Analysis**
- ***FlightlineOverlap***: Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping flight lines in each grid cell.
- ***LidarElevationSlice***: Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.
- ***LidarGroundPointFilter***: Identifies ground points within LiDAR dataset.
- ***LidarHillshade***: Calculates a hillshade value for points within a LAS file and stores these data in the RGB field.
- ***LidarInfo***: Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.
- ***LidarJoin***: Joins multiple LiDAR (LAS) files into a single LAS file.
- ***LidarTophatTransform***: Performs a tophat transform on a Lidar dataset.
- ***NormalVectors***: Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.

**Terrain Analysis**
- ***Aspect***: Calculates an aspect raster from an input DEM.
- ***DevFromMeanElev***: Calculates deviation from mean elevation.
- ***ElevPercentile***: Calculates the elevation percentile raster from a DEM.
- ***FillMissingData***: Fills nodata holes in a DEM.
- ***Hillshade***: Calculates a hillshade raster from an input DEM.
- ***PercentElevRange***: Calculates percent of elevation range from a DEM.
- ***PlanCurvature***: Calculates a plan (contour) curvature raster from an input DEM.
- ***ProfileCurvature***: Calculates a profile curvature raster from an input DEM.
- ***RelativeAspect***: Calculates relative aspect (relative to a user-specified direction) from an input DEM.
- ***RelativeTopographicPosition***: Calculates the relative topographic position index from a DEM.
- ***RuggednessIndex***: Calculates the Riley et al.'s (1999) terrain ruggedness index from an input DEM.
- ***RemoveOffTerrainObjects***: Removes off-terrain objects from a raster digital elevation model (DEM).
- ***Slope***: Calculates a slope raster from an input DEM.
- ***TangentialCurvature***: Calculates a tangential curvature raster from an input DEM.
- ***TotalCurvature***: Calculates a total curvature raster from an input DEM.

## Contributing

1. Fork the larger Whitebox project (in which whitebox-tools exists) ( https://github.com/jblindsay/whitebox-geospatial-analysis-tools )
2. Create your feature branch (git checkout -b my-new-feature)
3. Commit your changes (git commit -am 'Add some feature')
4. Push to the branch (git push origin my-new-feature)
5. Create a new Pull Request

## Contributors

- [jblindsay](https://github.com/jblindsay) Dr. John Lindsay - creator, maintainer

## License

The whitebox-tools library is distributed under the [MIT](LICENSE) license.

## Known Issues

- Currently GeoTIFF files can be read but cannot be written. This will hopefully be resolved soon.
- There is no support for reading, writing, or analyzing vector data yet. Plans include native support for the ESRI Shapefile format.