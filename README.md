# whitebox-tools

- [Desciption](#description)
- [Installation](#installation)
- [Usage](#usage)
- [Available Tools](#available-tools)
- [Contributing](#contributing)
- [Contributors](#contributors)
- [License](#license)

## Description
**whitebox-tools** is a library, developed using the Rust programming language, for analyzing geospatial data. Although it is intended to serve as a source of plugin tools for the [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/) open-source GIS project, the tools contained in this library are stand-alone and can be run outside of the larger Whitebox GAT project. See [Usage](#usage) for further details.

## Installation

Ensure that the latest stable version of the [Rust programming language](https://www.rust-lang.org) compiler is installed on your machine. Fork the GitHub repository then run the build.py Python script. The whitebox-tools.exe executable file will be located within the /target/release/ folder. 

## Usage
For examples of how to call functions and run tools from *whitebox-tools*, see the *whitebox_example.py* Python script, which itself uses the *whitebox_tools.py* script as an interface for interacting with the executable file. The *whitebox_tools.py* script calls the executable using subprocesses rather than as a dynamic library. Future versions may compile the library as a dynamic shared object if this is preferred.

## Available Tools

The library currently contains the following tools:

- ***Aspect***: Calculates an aspect raster from an input DEM.
- ***D8Pointer***: Calculates a D8 flow pointer raster from an input DEM.
- ***DevFromMeanElev***: Calculates deviation from mean elevation.
- ***ElevPercentile***: Calculates the elevation percentile raster from a DEM.
- ***FillMissingData***: Fills nodata holes in a DEM.
- ***FlightlineOverlap***: Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping flight lines in each grid cell.
- ***Hillshade***: Calculates a hillshade raster from an input DEM.
- ***LidarElevationSlice***: Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.
- ***LidarGroundPointFilter***: Identifies ground points within LiDAR dataset.
- ***LidarHillshade***: Calculates a hillshade value for points within a LAS file and stores these data in the RGB field.
- ***LidarInfo***: Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.
- ***LidarJoin***: Joins multiple LiDAR (LAS) files into a single LAS file.
- ***LidarTophatTransform***: Performs a tophat transform on a Lidar dataset.
- ***PercentElevRange***: Calculates percent of elevation range from a DEM.
- ***PlanCurvature***: Calculates a plan (contour) curvature raster from an input DEM.
- ***ProfileCurvature***: Calculates a profile curvature raster from an input DEM.
- ***RelativeTopographicPosition***: Calculates the relative topographic position index from a DEM.
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
