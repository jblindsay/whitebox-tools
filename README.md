![WhiteboxTools](./img/WhiteboxToolsLogoBlue.png)

<!--# WhiteboxTools-->

*Bringing the power of Whitebox GAT to the world at large*

- [Desciption](#description)
- [Installation](#installation)
- [Usage](#usage)
- [Available Tools](#available-tools)
- [Supported Data Formats](#supported-data-formats)
- [Contributing](#contributing)
- [Contributors](#contributors)
- [License](#license)
- [Known Issues](#known-issues)
- [Frequently Asked Questions](#frequently-asked-questions)
    * [Do I need Whitebox GAT to use WhiteboxTools?](#do-i-need-whitebox-gat-to-use-whiteboxtools)
    * [Why is WhiteboxTools Programmed in Rust?](#why-is-whiteboxtools-programmed-in-rust)
    * [How does WhiteboxTools' design philosophy differ?](#how-does-whiteboxtools-design-philosophy-differ)
    * [How do I request a tool be added?](#how-do-i-request-a-tool-be-added)
    * [Can WhiteboxTools be incorporated into other software and open-source GIS projects?](#can-whiteboxtools-be-incorporated-into-other-software-and-open-source-gis-projects)
    * [Do I need Rust installed on my computer to run WhiteboxTools?](#do-i-need-rust-installed-on-my-computer-to-run-whiteboxtools)
    * [What platforms does WhiteboxTools support?](#what-platforms-does-whiteboxtools-support)
    * [What are the system requirements?](#what-are-the-system-requirements)
    * [Are pre-compiled executables of WhiteboxTools available?](#are-pre-compiled-executables-of-whiteboxtools-available)

## Description

**WhiteboxTools** is an advanced geospatial data analysis engine. The library has been developed using the [Rust](https://www.rust-lang.org/en-US/) programming language, a very performant and safe systems language often viewed as a modern replacement for C/C++. Although *WhiteboxTools* is intended to serve as a source of plugin tools for the [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/) open-source GIS project, the tools contained in the library are stand-alone and can run outside of the larger Whitebox GAT project. See [Usage](#usage) for further details. There have been a large number of requests to call *Whitebox GAT* tools and functionality from outside of the Whitebox user-interface (e.g. from Python automation scripts). *WhiteboxTools* is intended to meet these usage requirements. Eventually most of the approximately 400 tools contained within *Whitebox GAT* will be ported to *WhiteboxTools*. In addition to separating the processing capabilities and the user-interface (and thereby reducing the reliance on Java), this migration should significantly improve processing efficiency. This is because Rust is generally [faster than the equivalent Java code](http://benchmarksgame.alioth.debian.org/u64q/compare.php?lang=rust&lang2=java) and because many of the *WhiteboxTools* functions are designed to process data in parallel wherever possible. In contrast, the older Java codebase included largely single-threaded applications. 

The *WhiteboxTools* project is related to the [*GoSpatial*](https://github.com/jblindsay/go-spatial) project, which has similar goals but is designed using the Go programming language instead of Rust. Both projects are currently considered to be experimental.

## Installation

*WhiteboxTools* is a stand-alone executable command-line program with no actual installation. To compile the latest development version of *WhiteboxTools* from source files, ensure that the latest stable version of the [Rust programming language](https://www.rust-lang.org) compiler is installed on your machine. Fork the *WhiteboxTools* GitHub repository and then run the *build.py* Python script. To run the build script, type the following command into a terminal, after having changed the terminal working directory to the *WhiteboxTools* folder:

```
>> python build.py
```

Compilation can take several minutes. The whitebox-tools.exe executable file will be located within the ```/target/release/``` folder. Once the project has reached the 1.0 milestone (stable), pre-compiled binaries for *WhiteboxTools* will be distributed with releases of *Whitebox GAT* GIS for each of the supported platforms. Until this time, you will need to compile the executable from source files.

Be sure to follow the instructions for installing Rust carefully. In particular, if you are installing on MS Windows, you must have a linker installed prior to installing the Rust compiler (rustc). The Rust webpage recommends either the **MS Visual C++ 2015 Build Tools** or the GNU equivalent and offers details for each installation approach. You should also consider using **RustUp** to install the Rust compiler. Ultimately, you should not have to interact with Rust directly, but rather the build script will do this for you.

## Usage

*WhiteboxTools* is a command-line program and can be run either by calling it, with appropriate commands and arguments, from a terminal application, or, more conveniently, by calling it from a script. The following commands are recognized:

| Command        | Description                                                                  |
| -------------- | ---------------------------------------------------------------------------- |
| --cd, --wd     | Changes the working directory; used in conjunction with --run flag.          |
| -l, --license  | Prints the whitebox-tools license.                                           |
| --listtools    | Lists all available tools, with tool descriptions                            |
| -r, --run      | Runs a tool; used in conjuction with --cd flag; -r="LidarInfo".              |
| --toolhelp     | Prints the help associated with a tool; --toolhelp="LidarInfo".              |
| -h, --help     | Prints help information.                                                     |

Generally, the Unix convention is that single-letter arguments (options) use a single dash (e.g. -h) while word-arguments (longer, more descriptive argument names) use double dashes (e.g. --help). The same rule is used for passing arguments to tools as well. Use the *--toolhelp* argument to print information about a specific tool (e.g. --toolhelp=Clump). Tool names can be specified either using the snake_case or CamelCase convention (e.g. *lidar_info* or *LidarInfo*).

For examples of how to call functions and run tools from *WhiteboxTools*, see the *whitebox_example.py* Python script, which itself uses the *whitebox_tools.py* script as an interface for interacting with the executable file. The *whitebox_tools.py* script calls the executable using subprocesses rather than as a dynamic library. Future versions may compile the library as a dynamic shared object if this is preferred.

**Example command prompt:**

```
>>./whitebox_tools --wd='/Users/johnlindsay/Documents/data/' --run=DevFromMeanElev --input='DEM clipped.dep' --output='DEV raster.dep' -v
```

Notice the quotation marks (single or double) used around directories and filenames, and string tool arguments in general. Use the '-v' flag (run in verbose mode) to force the tool print output to the command prompt. Please note that the whitebox_tools executable file must have permission to be executed; on some systems, this may require setting special permissions. The '>>' is shorthand for the command prompt and is not intended to be typed. Also, the above example uses the forward slash character (/), the directory path separator used on unix based systems. On Windows, users should use the back slash character (\) instead.

**Example Python script:**

```Python
import os
import sys
from whitebox_tools import WhiteboxTools

# Set the WhiteboxTools executable directory
# Change this to point to where you have the whitebox_tools.exe file!
wb_dir = os.path.dirname(os.path.abspath(__file__)) + "/target/release/"
wbt = WhiteboxTools()
wbt.set_whitebox_dir(wb_dir)

# Prints the WhiteboxTools help...a listing of available commands
print(wbt.help())

# Prints the WhiteboxTools license
print(wbt.license())

# Prints the WhiteboxTools version
print("Version information: {}".format(wbt.version()))

# List all available tools in WhiteboxTools
print(wbt.list_tools())

# Retrieve the help information for running the ElevPercentile tool
print(wbt.tool_help("ElevPercentile"))

# Sets verbose mode (True or False). Most tools will suppress output (e.g. updating
# progress) when verbose mode is False. The default is True
# wbt.set_verbose_mode(False) # uncomment me to suppress tool output

# Set the working directory; needed to specify complete file names (with paths) to tools that you run.
wbt.set_working_dir(os.path.dirname(os.path.abspath(__file__)) + "/testdata/")

tool_name = "ElevPercentile"
args = ["--input=\"DEM.dep\"",
        "--output=\"DEV_101.dep\"",
        "--filter=101"]

# Run the tool and check the return value
if wbt.run_tool(tool_name, args, callback) != 0:
    print("ERROR running {}".format(name))

```

## Available Tools

Eventually most of *Whitebox GAT's* approximately 400 tools [will be ported](tool_porting.md) to *WhiteboxTools*, although this is an immense task. Support for vector data (Shapefile/GeoJSON) reading/writing and a topological analysis library (like the Java Topology Suite) will need to be added in order to port the tools involving vector spatial data. Opportunities to parallelize algorithms will be sought during porting. All new plugin tools will be added to *Whitebox GAT* using this library of functions. 

The library currently contains the following 192 tools:

**Data Tools**
- ***ConvertNodataToZero***: Converts nodata values in a raster to zero.
- ***ConvertRasterFormat***: Converts raster data from one format to another.
- ***NewRasterFromBase***: Creates a new raster using a base image.

**GIS Analysis**
- ***AverageOverlay***: Calculates the average for each grid cell from a group of raster images.
- ***BufferRaster***: Maps a distance-based buffer around each non-background (non-zero/non-nodata) grid cell in an input image.
- ***Clump***: Groups cells that form physically discrete areas, assigning them unique identifiers.
- ***CostAllocation***: Identifies the source cell to which each grid cell is connected by a least-cost pathway in a cost-distance analysis.
- ***CostDistance***: Performs cost-distance accumulation on a cost surface and a group of source cells.
- ***CostPathway***: Performs cost-distance pathway analysis using a series of destination grid cells.
- ***CreatePlane***: Creates a raster image based on the equation for a simple plane.
- ***EuclideanAllocation***: Assigns grid cells in the output raster the value of the nearest target cell in the input image, measured by the Shih and Wu (2004) Euclidean distance transform.
- ***EuclideanDistance***: Calculates the Shih and Wu (2004) Euclidean distance transform.
- ***HighestPosition***: Identifies the stack position of the maximum value within a raster stack on a cell-by-cell basis.
- ***LowestPosition***: Identifies the stack position of the minimum value within a raster stack on a cell-by-cell basis.
- ***MaxAbsoluteOverlay***: Evaluates the maximum absolute value for each grid cell from a stack of input rasters.
- ***MaxOverlay***: Evaluates the maximum value for each grid cell from a stack of input rasters.
- ***MinAbsoluteOverlay***: Evaluates the minimum absolute value for each grid cell from a stack of input rasters.
- ***MinOverlay***: Evaluates the minimum value for each grid cell from a stack of input rasters.
- ***PercentEqualTo***: Calculates the percentage of a raster stack that have cell values equal to an input on a cell-by-cell basis.
- ***PercentGreaterThan***: Calculates the percentage of a raster stack that have cell values greater than an input on a cell-by-cell basis.
- ***PercentLessThan***: Calculates the percentage of a raster stack that have cell values less than an input on a cell-by-cell basis.
- ***PickFromList***: Outputs the value from a raster stack specified by a position raster.
- ***ReclassEqualInterval***: Reclassifies the values in a raster image based on equal-ranges.
- ***WeightedSum***: Performs a weighted-sum overlay on multiple input raster images.

**Hydrological Analysis**
- ***AverageUpslopeFlowpathLength***: Measures the average length of all upslope flowpaths draining each grid cell.
- ***Basins***: Identifies drainage basins that drain to the DEM edge.
- ***BreachDepressions***: Breaches all of the depressions in a DEM. This should be preferred over depression filling in most cases.
- ***D8FlowAccumulation***: Calculates a D8 flow accumulation raster from an input DEM.
- ***D8Pointer***: Calculates a D8 flow pointer raster from an input DEM.
- ***DepthInSink***: Measures the depth of sinks (depressions) in a DEM.
- ***DInfFlowAccumulation***: Calculates a D-infinity flow accumulation raster from an input DEM.
- ***DInfPointer***: Calculates a D-infinity flow pointer (flow direction) raster from an input DEM.
- ***DownslopeDistanceToStream***: Measures distance to the nearest downslope stream cell.
- ***DownslopeFlowpathLength***: Calculates the downslope flowpath length from each cell to basin outlet.
- ***ElevationAboveStream***: Calculates the elevation of cells above the nearest downslope stream cell.
- ***FD8FlowAccumulation***: Calculates a FD8 flow accumulation raster from an input DEM.
- ***FD8Pointer***: Calculates an FD8 flow pointer raster from an input DEM.
- ***FloodOrder***: Assigns each DEM grid cell its order in the sequence of inundations that are encountered during a search starting from the edges, moving inward at increasing elevations.
- ***FlowLengthDiff***: Calculates the local maximum absolute difference in downslope flowpath length, useful in mapping drainage divides and ridges.
- ***FillDepressions***: Fills all of the depressions in a DEM. Depression breaching should be preferred in most cases.
- ***FillSingleCellPits***: Raises pit cells to the elevation of their lowest neighbour.
- ***FindNoFlowCells***: Finds grid cells with no downslope neighbours.
- ***FindParallelFlow***: Finds areas of parallel flow in D8 flow direction rasters.
- ***JensonSnapPourPoints***: Moves outlet points used to specify points of interest in a watershedding operation to the nearest stream cell.
- ***MaxUpslopeFlowpathLength***: Measures the maximum length of all upslope flowpaths draining each grid cell.
- ***NumInflowingNeighbours***: Computes the number of inflowing neighbours to each cell in an input DEM based on the D8 algorithm.
- ***Sink***: Identifies the depressions in a DEM, giving each feature a unique identifier.
- ***SnapPourPoints***: Moves outlet points used to specify points of interest in a watershedding operation to the cell with the highest flow accumulation in its neighbourhood.
- ***Subbasins***: Identifies the catchments, or sub-basin, draining to each link in a stream network.
- ***TraceDownslopeFlowpaths***: Traces downslope flowpaths from one or more target sites (i.e. seed points).
- ***Watershed***: Identifies the watershed, or drainage basin, draining to a set of target cells.

**Image Analysis**
- ***AdaptiveFilter***: Performs an adaptive filter on an image.
- ***BilateralFilter***: A bilateral filter is an edge-preserving smoothing filter introduced by Tomasi and Manduchi (1998).
- ***Closing***: A closing is a mathematical morphology operating involving an erosion (min filter) of a dilation (max filter) set.
- ***ConservativeSmoothingFilter***: Performs a conservative smoothing filter on an image.
- ***DiffOfGaussianFilter***: Performs a Difference of Gaussian (DoG) filter on an image.
- ***DiversityFilter***: Assigns each cell in the output grid the number of different values in a moving window centred on each grid cell in the input raster.
- ***EmbossFilter***: Performs an emboss filter on an image, similar to a hillshade operation.
- ***FlipImage***: Reflects an image in the vertical or horizontal axis.
- ***GaussianFilter***: Performs a Gaussian filter on an image.
- ***HighPassFilter***: Performs a high-pass filter on an input image.
- ***IntegralImage***: Transforms an input image (summed area table) into its integral image equivalent.
- ***LaplacianFilter***: Performs a Laplacian filter on an image.
- ***LaplacianOfGaussianFilter***: Performs a Laplacian-of-Gaussian (LoG) filter on an image.
- ***LineDetectionFilter***: Performs a line-detection filter on an image.
- ***LineThinning***: Performs line thinning a on Boolean raster image; intended to be used with the RemoveSpurs tool.
- ***MajorityFilter***: Assigns each cell in the output grid the most frequently occuring value (mode) in a moving window centred on each grid cell in the input raster.
- ***MaximumFilter***: Assigns each cell in the output grid the maximum value in a moving window centred on each grid cell in the input raster.
- ***MeanFilter***: Performs a mean filter (low-pass filter) on an input image.
- ***MinimumFilter***: Assigns each cell in the output grid the minimum value in a moving window centred on each grid cell in the input raster.
- ***OlympicFilter***: Performs an olympic smoothing filter on an image.
- ***Opening***: An opening is a mathematical morphology operating involving a dilation (max filter) of an erosion (min filter) set.
- ***NormalizedDifferenceVegetationIndex***: Calculates the normalized difference vegetation index (NDVI) from near-infrared and red imagery.
- ***PercentileFilter***: Performs a percentile filter on an input image.
- ***PrewittFilter***: Performs a Prewitt edge-detection filter on an image.
- ***RangeFilter***: Assigns each cell in the output grid the range of values in a moving window centred on each grid cell in the input raster.
- ***RemoveSpurs***: Removes the spurs (prunning operation) from a Boolean line image.; intended to be used on the output of the LineThinning tool.
- ***RobertsCrossFilter***: Performs a Robert's cross edge-detection filter on an image.
- ***ScharrFilter***: Performs a Scharr edge-detection filter on an image.
- ***SobelFilter***: Performs a Sobel edge-detection filter on an image.
- ***StandardDeviationFilter***: Assigns each cell in the output grid the standard deviation of values in a moving window centred on each grid cell in the input raster.
- ***ThickenRasterLine***: Thickens single-cell wide lines within a raster image.
- ***TophatTransform***: Performs either a white or black top-hat transform on an input image
- ***TotalFilter***: Performs a total filter on an input image.

**LiDAR Analysis**
- ***BlockMaximum***: Creates a block-maximum raster from an input LAS file.
- ***BlockMinimum***: Creates a block-minimum raster from an input LAS file.
- ***FlightlineOverlap***: Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping flight lines in each grid cell.
- ***LidarElevationSlice***: Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.
- ***LidarGroundPointFilter***: Identifies ground points within LiDAR dataset.
- ***LidarIdwInterpolation***: Interpolates LAS files using an inverse-distance weighted (IDW) scheme.
- ***LidarHillshade***: Calculates a hillshade value for points within a LAS file and stores these data in the RGB field.
- ***LidarInfo***: Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.
- ***LidarJoin***: Joins multiple LiDAR (LAS) files into a single LAS file.
- ***LidarNearestNeighbourGridding***: Grids LAS files using nearest-neighbour scheme.
- ***LidarPointDensity***: Calculates the spatial pattern of point density fore a LiDAR data set.
- ***LidarTile***: Tiles a LiDAR LAS file into multiple LAS files.
- ***LidarTophatTransform***: Performs a white top-hat transform on a Lidar dataset.
- ***NormalVectors***: Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.

**Mathematical and Statistical Analysis**
- ***AbsoluteValue***: Calculates the absolute value of every cell in a raster.
- ***Add***: Performs an addition operation on two rasters or a raster and a constant value.
- ***And***: Performs a logical AND operator on two Boolean raster images.
- ***ArcCos***: Returns the inverse cosine (arccos) of each values in a raster.
- ***ArcSin***: Returns the inverse sine (arcsin) of each values in a raster.
- ***ArcTan***: Returns the inverse tangent (arctan) of each values in a raster.
- ***Atan2***: Returns the 2-argument inverse tangent (atan2).
- ***Ceil***: Returns the smallest (closest to negative infinity) value that is greater than or equal to the values in a raster.
- ***Cos***: Returns the cosine (cos) of each values in a raster.
- ***Cosh***: Returns the hyperbolic cosine (cosh) of each values in a raster.
- ***Decrement***: Decreases the values of each grid cell in an input raster by 1.0.
- ***Divide***: Performs a division operation on two rasters or a raster and a constant value.
- ***EqualTo***: Performs a equal-to comparison operation on two rasters or a raster and a constant value.
- ***Exp***: Returns the exponential (base e) of values in a raster.
- ***Exp2***: Returns the exponential (base 2) of values in a raster.
- ***Floor***: Returns the largest (closest to positive infinity) value that is greater than or equal to the values in a raster.
- ***GreaterThan***: Performs a greater-than comparison operation on two rasters or a raster and a constant value.
- ***Increment***: Increases the values of each grid cell in an input raster by 1.0.
- ***IntegerDivision***: Performs an integer division operation on two rasters or a raster and a constant value.
- ***IsNoData***: Identifies NoData valued pixels in an image.
- ***LessThan***: Performs a less-than comparison operation on two rasters or a raster and a constant value.
- ***Log10***: Returns the base-10 logarithm of values in a raster.
- ***Log2***: Returns the base-2 logarithm of values in a raster.
- ***Ln***: Returns the natural logarithm of values in a raster.
- ***Max***: Performs a MAX operation on two rasters or a raster and a constant value.
- ***Min***: Performs a MIN operation on two rasters or a raster and a constant value.
- ***Modulo***: Performs a modulo operation on two rasters or a raster and a constant value.
- ***Multiply***: Performs a multiplication operation on two rasters or a raster and a constant value.
- ***Negate***: Changes the sign of values in a raster or the 0-1 values of a Boolean raster.
- ***Not***: Performs a logical NOT operator on two Boolean raster images.
- ***NotEqualTo***: Performs a not-equal-to comparison operation on two rasters or a raster and a constant value.
- ***Or***: Performs a logical OR operator on two Boolean raster images.
- ***Power***: Raises the values in grid cells of one rasters, or a constant value, by values in another raster or constant value.
- ***Quantiles***: Tranforms raster values into quantiles.
- ***RandomField***: Creates an image containing random values.
- ***Reciprocal***: Returns the reciprocal (i.e. 1 / z) of values in a raster.
- ***Round***: Rounds the values in an input raster to the nearest integer value.
- ***Sin***: Returns the sine (sin) of each values in a raster.
- ***Sinh***: Returns the hyperbolic sine (sinh) of each values in a raster.
- ***Square***: Squares the values in a raster.
- ***SquareRoot***: Returns the square root of the values in a raster.
- ***Subtract***: Performs a subtraction operation on two rasters or a raster and a constant value.
- ***Tan***: Returns the tangent (tan) of each values in a raster.
- ***Tanh***: Returns the hyperbolic tangent (tanh) of each values in a raster.
- ***ToDegrees***: Converts a raster from radians to degrees.
- ***ToRadians***: Converts a raster from degrees to radians.
- ***Truncate***: Truncates the values in a raster to the desired number of decimal places.
- ***Xor***: Performs a logical XOR operator on two Boolean raster images.
- ***ZScores***: Standardizes the values in an input raster by converting to z-scores.

**Stream Network Analysis**
- ***ExtractStreams***: Extracts stream grid cells from a flow accumulation raster.
- ***ExtractValleys***: Identifies potential valley bottom grid cells based on local topolography alone.
- ***FarthestChannelHead***: Calculates the distance to the furthest upstream channel head for each stream cell.
- ***FindMainStem***: Finds the main stem, based on stream lengths, of each stream network.
- ***HackStreamOrder***: Assigns the Hack stream order to each link in a stream network.
- ***HortonStreamOrder***: Assigns the Horton stream order to each link in a stream network.
- ***RemoveShortStreams***: Removes short first-order streams from a stream network.
- ***ShreveStreamMagnitude***: Assigns the Shreve stream magnitude to each link in a stream network.
- ***StrahlerStreamOrder***: Assigns the Strahler stream order to each link in a stream network.
- ***StreamLinkClass***: Identifies the exterior/interior links and nodes in a stream network.
- ***StreamLinkIdentifier***: Assigns a unique identifier to each link in a stream network.
- ***StreamLinkLength***: Estimates the length of each link (or tributary) in a stream network.
- ***StreamLinkSlope***: Estimates the average slope of each link (or tributary) in a stream network.
- ***StreamSlopeContinuous***: Estimates the slope of each grid cell in a stream network.
- ***TopologicalStreamOrder***: Assigns each link in a stream network its topological order.
- ***TotalLengthOfUpstreamChannels***: Calculates the total length of channels upstream.
- ***TributaryIdentifier***: Assigns a unique identifier to each tributary in a stream network.

**Terrain Analysis**
- ***Aspect***: Calculates an aspect raster from an input DEM.
- ***DevFromMeanElev***: Calculates deviation from mean elevation.
- ***DiffFromMeanElev***: Calculates difference from mean elevation (equivalent to a high-pass filter).
- ***DirectionalRelief***: Calculates relief for cells in an input DEM for a specified direction.
- ***ElevPercentile***: Calculates the elevation percentile raster from a DEM.
- ***FetchAnalysis***: Performs an analysis of fetch or upwind distance to an obstacle.
- ***FillMissingData***: Fills nodata holes in a DEM.
- ***Hillshade***: Calculates a hillshade raster from an input DEM.
- ***MaxBranchLength***: Branch length is used to map drainage divides or ridge lines.
- ***MaxDownslopeElevChange***: Calculates the maximum downslope change in elevation between a grid cell and its eight downslope neighbors.
- ***MinDownslopeElevChange***: Calculates the minimum downslope change in elevation between a grid cell and its eight downslope neighbors.
- ***HorizonAngle***: Calculates horizon angle (maximum upwind slope) for each grid cell in an input DEM.
- ***NumDownslopeNeighbours***: Calculates the number of downslope neighbours to each grid cell in a DEM.
- ***NumUpslopeNeighbours***: Calculates the number of upslope neighbours to each grid cell in a DEM.
- ***PennockLandformClass***: Classifies hillslope zones based on slope, profile curvature, and plan curvature.
- ***PercentElevRange***: Calculates percent of elevation range from a DEM.
- ***PlanCurvature***: Calculates a plan (contour) curvature raster from an input DEM.
- ***ProfileCurvature***: Calculates a profile curvature raster from an input DEM.
- ***RelativeAspect***: Calculates relative aspect (relative to a user-specified direction) from an input DEM.
- ***RelativeStreamPowerIndex***: Calculates the relative stream power index.
- ***RelativeTopographicPosition***: Calculates the relative topographic position index from a DEM.
- ***RuggednessIndex***: Calculates the Riley et al.'s (1999) terrain ruggedness index from an input DEM.
- ***RemoveOffTerrainObjects***: Removes off-terrain objects from a raster digital elevation model (DEM).
- ***SedimentTransportIndex***: Calculates the sediment transport index.
- ***Slope***: Calculates a slope raster from an input DEM.
- ***TangentialCurvature***: Calculates a tangential curvature raster from an input DEM.
- ***TotalCurvature***: Calculates a total curvature raster from an input DEM.
- ***WetnessIndex***: Calculates the topographic wetness index, Ln(A / tan(slope)).

To retrieve detailed information about a tool's input arguments and example usage, either use the *--toolhelp* command from the terminal, or the *tool_help('tool_name')* function from the *whitebox_tools.py* script.

## Supported Data Formats
The **WhiteboxTools** library can currently support read/writing raster data in [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/), ESRI (ArcGIS) ASCII and binary (*.flt & *.hdr), GRASS GIS, Idrisi, SAGA GIS (binary and ASCII), and Surfer 7 data formats. Currently GeoTiff files can be read but not written, although work is underway to add data writing capabilities. The library is primarily tested using Whitebox raster data sets and if you encounter issues when reading/writing data in other formats, you should report the problem to the [author](#contributors). Please note that there are no plans to incorportate third-party libraries, like [GDAL](http://www.gdal.org), in the project given the design goal of keeping a pure (or as close as possilbe) Rust codebase. LiDAR data can be read/written in the common [LAS](https://www.asprs.org/committee-general/laser-las-file-format-exchange-activities.html) data format. Zipped LAS formats (LAZ) and ESRI LiDAR formats are not currently supported. At present, there is no ability to read or write vector geospatial data. Support for Shapefile, GeoJSON, and other common vector formats will eventually be added to the library.

## Contributing

If you would like to contribute to the project as a developer, follow these instructions to get started:

1. Fork the larger Whitebox project (in which whitebox-tools exists) ( https://github.com/jblindsay/whitebox-geospatial-analysis-tools )
2. Create your feature branch (git checkout -b my-new-feature)
3. Commit your changes (git commit -am 'Add some feature')
4. Push to the branch (git push origin my-new-feature)
5. Create a new Pull Request

**TODO**
Describe the process of integrating a new tool into the library.

If you would like to contribute financial support for the project, please contact [John Lindsay](http://www.uoguelph.ca/~hydrogeo/index.html). We also welcome contributions in the form of media exposure. If you have written an article or blog about *WhiteboxTools* please let us know about it.

## Contributors

- [jblindsay](https://github.com/jblindsay) Dr. John Lindsay ([webpage](http://www.uoguelph.ca/~hydrogeo/index.html)) - creator, maintainer

## License

The **WhiteboxTools** library is distributed under the [MIT license](LICENSE), a permissive open-source (free software) license.

## Known Issues

- Currently GeoTIFF files can be read but cannot be written. This will hopefully be resolved soon.
- There is no support for reading, writing, or analyzing vector data yet. Plans include native support for the ESRI Shapefile format.
- Compressed LAS files (LAZ) are not supported.
- File directories cannot contain apostrophes (') as they will be interpreted in the arguments array as single quoted strings.

## Frequently Asked Questions

### Do I need Whitebox GAT to use WhiteboxTools?

No you do not. You can call the tools contained within *WhiteboxTools* completely independent from the *Whitebox GAT* user interface. In fact, you can interact with the tools using Python scripting or directly, using a terminal application (command prompt). See [Usage](#usage) for further details.

### Why is WhiteboxTools programmed in Rust?

I spent a long time evaluating potential programming language for future development efforts for the *Whitebox GAT* project. My most important criterion for a language was that it compile to native code, rather than target the Java virtual machine (JVM). I have been keen to move Whitebox GAT away from Java because of some of the challenges that supporting the JVM has included for many Whitebox users. The language should be fast and productive--Java is already quite fast, but if I am going to change development languages, I would like a performance boost. Furthermore, given that many, though not all, of the algorithms used for geospatial analysis scale well with concurrent (parallel) implementations, I favoured languages that offerred easy and safe concurrent programming. Although many would consider C/C++ for this work, I was looking for a modern and safe language. Fortunately, we are living through a renaissance period in programming language development and there are many newer languages that fit the bill nicely. Over the past two years, I considered each of Go, Rust, D, Nim, and Crystal for Whitebox development and ultimately decided on Rust. [See [*GoSpatial*](https://github.com/jblindsay/go-spatial) and [*lidario*](https://github.com/jblindsay/lidario).]

Each of the languages I examined has its own advantages of disadvantages, so why Rust? It's a combination of factors that made it a compelling option for this project. Compared with many on the list, Rust is a mature language with a vibrant user community. Like C/C++, it's a high-performance and low-level language that allows for complete control of the system. However, Rust is also one of the safest languages, meaning that I can be confident that *WhiteboxTools* will not contain common bugs, such as memory use-after-release, memory leaks and race conditions within concurrent code. Importantly, and quite uniquely, this safty is achieved in the Rust language without the use of a garbage collector (automatic memory management). Garbage collectors can be great, but they do generally come with a certain efficiency trade-off that Rust does not have. The other main advantage of Rust's approach to memory management is that it allows for  a level of interaction with scripting languages (e.g. Python) that is quite difficult to do in garbage collected languages. Although **WhiteboxTools** is currently set up to use an automation approach to interacting with Python code that calls it, I like the fact that I have the option to create a *WhiteboxTools* shared library. 

Not everything with Rust is perfect however. It is still a very young language and there are many pieces still mising from its ecosystem. Futhermore, it is not the easiest language to learn, particularly for people who are inexperienced with programming. This may limit my ability to attract other programers to the Whitebox project, which would be unfortunate. However, overall, Rust was the best option for this particular application.

### How does WhiteboxTools' design philosophy differ?

*Whitebox GAT* is frequently praised for its consistent design and ease of use. Like *Whitebox GAT*, *WhiteboxTools* follows the convention of *one tool for one function*. For example, in *WhiteboxTools* assigning the links in a stream channel network their Horton, Strahler, Shreve, or Hack stream ordering numbers requires running separate tools (i.e. *HortonStreamOrder*, *StrahlerStreamOrder*, *ShreveStreamMagnitude*, and *HackStreamOrder*). By contrast, in GRASS GIS<sup>1</sup> and ArcGIS single tools (i.e. the *r.stream.order* and *Stream Order* tools respectively) can be configured to output different channel ordering schemes. The *WhiteboxTools* design is intended to simplify the user experience and to make it easier to find the right tool for a task. With more specific tool names that are reflective of their specific purposes, users are not as reliant on reading help documentation to identify the tool for the task at hand. Similarly, it is not uncommon for tools in other GIS to have multiple outputs. For example, in GRASS GIS the *r.slope.aspect* tool can be configured to output slope, aspect, profile curvature, plan curvature, and several other common terrain surface derivatives. Based on the *one tool for one function* design approach of *WhiteboxTools*, multiple outputs are indicative that a tool should be split into different, more specific tools. Are you more likely to go to a tool named *r.slope.aspect* or *TangentialCurvature* when you want to create a tangential curvature raster from a DEM? If you're new to the software and are unfamilar with it, probably the later is more obvious. The *WhiteboxTools* design approach also has the added benefit of simplifying the documentation for tools. The one downside to this design approach, however, is that it results (or will result) in a large number of tools, often with signifcant overlap in function. 

<sup>1</sup> NOTE: It's not my intent to criticize GRASS GIS, as I deeply respect the work that the GRASS developers have contributed. Rather, I am contrasting the consequences of *WhiteboxTools'* design philosophy to that of other GIS.

### How do I request a tool be added?

Eventually most of the tools in *Whitebox GAT* will be ported over to *WhiteboxTools* and all new tools will be added to this library as well. Naturally, this will take time. The order by which tools are ported is partly a function of ease of porting, existing infrastructure (i.e. raster and LiDAR tools will be ported first since their is currently no support in the library for vector I/O), and interest. If you are interested in making a tool a higher priority for porting, email [John Lindsay](http://www.uoguelph.ca/~hydrogeo/index.html).

### Can WhiteboxTools be incorporated into other software and open-source GIS projects?

*WhiteboxTools* was developed with the open-source GIS [Whitebox GAT](http://www.uoguelph.ca/~hydrogeo/Whitebox/index.html) in mind. That said, the tools can be accessed independently and so long as you abide by the terms of the [MIT license](#license), there is no reason why other software and GIS projects cannot use *WhiteboxTools* as well. In fact, this was one of the motivating factors for creating the library in the first place! Feel free to use *WhiteboxTools* as the geospatial analysis engine in your open-source software project.

### Do I need Rust installed on my computer to run WhiteboxTools?

No, you would only need Rust installed if you were compiling the WhiteboxTools codebase from source files. Eventually I will distribute compiled versions of the tools for various supported platforms. For now, however, you will need to compile the project yourself (see [Installation](#installation) for details). The compilation product (*whitebox_tools.exe* file) is a stand-alone executable that can be copied to and run on other computers that do not have Rust installed. Being natively compiled means that the executable file is system-dependent.

### What platforms does WhiteboxTools support?

*WhiteboxTools* is developed using the Rust programming language, which supports a [wide variety of platforms](https://forge.rust-lang.org/platform-support.html) including MS Windows, MacOS, and Linux operating systems and common chip architectures. Interestingly, Rust also supports mobile platforms, and *WhiteboxTools* should therefore be capable of targeting (although no testing has been completed in this regard to date). Nearly all development and testing of the software is currently carried out on MacOS and we cannot guarantee a bug-free performance on other platforms. In particularly, MS Windows is the most different from the other platforms and is therefore the most likely to encounter platform-specific bugs. If you encounter bugs in the software, please consider reporting an issue using the GitHub support for issue-tracking.

### What are the system requirements?

The answer to this question depends strongly on the type of analysis and data that you intend to process. However, generally we find performance to be optimal with a recommended minimum of 8-16GB of memory (RAM), a modern multi-core processor (e.g. 64-bit i5 or i7), and an solid-state-drive (SSD). It is likely that *WhiteboxTools* will have satisfactory performance on lower-spec systems if smaller datasets are being processed. Because *WhiteboxTools* reads entire raster datasets into system memory (for optimal performance, and in recognition that modern systems have increasingly larger amounts of fast RAM), this tends to be the limiting factor for the upper-end of data size successfully processed by the library. 64-bit operating systems are recommended and extensive testing has not been carried out on 32-bit OSs. See [**"What platforms does WhiteboxTools support?"**](#what-platforms-does-whiteboxtools-support) for further details on supported platforms.

### Are pre-compiled executables of WhiteboxTools available?

Once the project has reached the stable 1.0 milestone, pre-compiled binaries for *WhiteboxTools* will be distributed with releases of *Whitebox GAT* GIS for each of the supported platforms. Until this time, you will need to compile the executable from source files. See [Installation](#installation) for details.