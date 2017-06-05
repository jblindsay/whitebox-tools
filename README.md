# whitebox-tools

This is a library, developed using the Rust programming language, for analyzing geospatial data. Although it is intended to
serve as a source of plugin tools for the [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/) open-source GIS project, the tools contained in this library are
stand-alone and can be run outside of the larger Whitebox GAT project.

## Installation

Fork the GitHub repository then run the build.py Python script. The whitebox-tools.exe executable file will be located within
the /target/release/ folder. 


## Usage
For examples of how to call functions and run tools from *whitebox-tools*, see the *whitebox_example.py* Python script, which itself uses the *whitebox_tools.py* script as an interface for interacting with the executable file. The *whitebox_tools.py* script calls
the executable using subprocesses rather than as a dynamic library. Future versions may compile the library as a dynamic shared object
if this is preferred.

The library currently contains the following tools:

- ***dev_from_mean_elev***: Calculates deviation from mean elevation.
- ***elev_percentile***: Calculates the elevation percentile raster from a DEM.
- ***lidar_elevation_slice***: Outputs all of the points within a LiDAR (LAS) point file that lie between a specified
elevation range.
- ***lidar_flightline_overlap***: Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping
- flight lines in each grid cell.
- ***lidar_info***: Prints information about a LiDAR (LAS) dataset, including header, point return frequency,
and classification data and information about the variable length records (VLRs) and geokeys.
- ***lidar_join***: Joins multiple LiDAR (LAS) files into a single LAS file.
- ***percent_elev_range***: Calculates percent of elevation range from a DEM.
- ***relative_topographic_position***: Calculates the relative topographic position index from a DEM.
- ***remove_off_terrain_objects***: Removes off-terrain objects from a raster digital elevation model (DEM).


## Contributing

1. Fork the larger Whitebox project (in which whitebox-tools exists) ( https://github.com/jblindsay/whitebox-geospatial-analysis-tools )
2. Create your feature branch (git checkout -b my-new-feature)
3. Commit your changes (git commit -am 'Add some feature')
4. Push to the branch (git push origin my-new-feature)
5. Create a new Pull Request

## Contributors

- [jblindsay](https://github.com/jblindsay) John Lindsay - creator, maintainer

## License
[MIT](LICENSE)
