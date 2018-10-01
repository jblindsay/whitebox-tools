WhiteboxTools

The main tool library is contained in the whitebox_tools (or whitebox_tools.exe on 
MS Windows) file. This is a command-line program that can be run from a terminal, i.e. 
command prompt. For details on usage, change the working directory (cd) to this folder 
and type the following at the command prompt:

./whitebox_tools --help

The following commands are recognized:
--cd, --wd       Changes the working directory; used in conjunction with --run flag.
-h, --help       Prints help information.
-l, --license    Prints the whitebox-tools license.
--listtools      Lists all available tools. Keywords may also be used, --listtools slope.
-r, --run        Runs a tool; used in conjuction with --wd flag; -r="LidarInfo".
--toolbox        Prints the toolbox associated with a tool; --toolbox=Slope.
--toolhelp       Prints the help associated with a tool; --toolhelp="LidarInfo".
--toolparameters Prints the parameters (in json form) for a specific tool; --toolparameters="LidarInfo".
-v               Verbose mode. Without this flag, tool outputs will not be printed.
--viewcode       Opens the source code of a tool in a web browser; --viewcode="LidarInfo".
--version        Prints the version information.

Example Usage:
>> ./whitebox-tools -r=lidar_info --cd="/path/to/data/" -i=input.las --vlr --geokeys


The WhiteboxTools library may also be called from Python automation scripts. The 
whitebox_tools.py script can be used as an easy way of interfacing with the various 
commands. See the user manual for more deails. To use this script, simply use the 
following import:

from whitebox_tools import WhiteboxTools

wbt = WhiteboxTools() 
wbt.work_dir = "/path/to/data/" # Sets the Whitebox working directory

wbt.d_inf_flow_accumulation("DEM.dep", "output.dep", log=True)


Additionally, included in this directory is the WhiteboxTools Runner, a simple Tkinter 
user-interface that allows users to run the WhiteboxTools tools, with convenience for 
specifying tool parameters. To run this interface, simply type:

python3 wb_runner.py

Or, if Python 3 is the default Python intepreter:

python wb_runner.py

At the command prompt (after cd'ing to this folder, which contains the script).

WhiteboxTools is distributed under a permissive MIT open-source license. See LICENSE.txt 
for more details.

******************
* Release Notes: *
******************

Version 0.11.0 (XX-XX-2018)
- The following tools were added to the project:
    AddPointCoordinatesToTable
    CentroidVector
    CompactnessRatio
    ConstructVectorTIN
    ElongationRatio
    ExtendVectorLines
    HoleProportion
    LayerFootprint
    LidarConstructVectorTIN
    LidarTINGridding
    LinesToPolygons
    Medoid
    MinimumBoundingCircle
    MinimumBoundingEnvelope
    MultiPartToSinglePart
    PerimeterAreaRatio
    PolygonArea
    PolygonPerimeter
    RasterStreamsToVector
    RasterToVectorPoints
    RelatedCircumscribingCircle
    RemovePolygonHoles
    ShapeComplexityIndex
    SinglePartToMultiPart
    SmoothVectors
    SumOverlay
    TINGridding

- Added a minimum number of neighbours criteria in the neighbourhood search of the
  LidarGroundPointFilter tool. In this way, if the fixed-radius search yields fewer
  neighbours than this minimum neighbours threshold, a second kNN search is carried
  out to identify the k nearest neighbours. This can be preferable for cases where
  the point density varies significantly in the data set, e.g. in the case of 
  terrestrial LiDAR point clouds.
- The MinimumBoundingBox tool has been modified to take an optional minimization 
  criteria, including minimum box area, length, width, or perimeter.
- Fixed: Bug that resulted in a 0.5 m offset in the positioning of interpolated grids.
- Fixed: Viewshed tool now emits an intelligible error when the viewing station does 
  not overlap with the DEM.

Version 0.10.0 (16-09-2018)
- The following tools were added to the project:
    CreateHexagonalVectorGrid
    CreateRectangularVectorGrid
    DrainagePreservingSmoothing
    EliminateCoincidentPoints
    ExtractNodes
    HighPassMedianFilter
    LasToMultipointShapefile
    LidarHexBinning and VectorHexBinning
    LidarTileFootprint
    MaxDifferenceFromMean
    MinimumBoundingBox
    MinimumConvexHull
    PolygonLongAxis and PolygonShortAxis
    PolygonsToLines
    ReinitializeAttributeTable

- Refactoring of some data related to Point2D, and common algorithms (e.g. 
  point-in-poly, convex hull).
- Added unit tests to BoundingBox, point_in_poly, convex_hull, and elsewhere.
- Fixed a bug in LiDAR join related to tiles with fewer than two points. LAS files
  now issue a warning upon saving when they contain less than two points.
- The default callback can now be modified in whitebox_tools.py, such that
  a single custom callback can be used without having to specify it for each
  tool function call.
- Added initial support for getting projection ESPG and WKT info from LAS files 
  and GeoTiff data. This is the start of a more fullsome approach to handling
  spatial reference system information in the library.
- Fixed a bug in saving Shapefile m and z data.
- Fixed a bug that wouldn't allow the LidarIdwInterpolation and 
  LidarNearestNeighbourGridding tool to interpolate point classification data.
- LidarGroundPointFilter now has the ability to output a classified LAS file rather 
  than merely filtering non-ground points. Ground points are assigned classification
  values of 2 while non-ground points are classified as 1.
- Updated the LidarKappaIndex tool to use a NN-search to find matching points between
  the compared point clouds.
- Modified the FixedRadiusSearch structure to use 64-bit floats for storing coordinates.
  This impacts performance efficiency but is needed for the fine precision of 
  positional information found in terrestrial LiDAR data. FixedRadiusSearch structures
  have also had approximate kNN search methods added.


Version 0.9.0 (22-08-2018)
- Added the following tools:
    ExtractRasterValuesAtPoints
    FindLowestOrHighestPoints
    LidarThinHighDensity
    SelectTilesByPolygon
    StandardDeviationOfSlope
    
- Support has been added for writing Shapefile vector data.
- The SnapPourPoints and JensonSnapPourPoints tools have been modified to accept
  vector inputs and to produce vector outputs. This is more consistent with 
  the Watershed tool, which requires vector pour point data inputs.


Version 0.8.0 (30-05-2018)
- Added the following tools:
    CornerDetection
    FastAlmostGaussianFilter
    GaussianContrastStretch
    IdwInterpolation
    ImpoundmentIndex
    LidarThin
    StochasticDepressionAnalysis
    UnsharpMasking
    WeightedOverlay
- Modified some filters to take RGB inputs by operating on the intensity value. 
  These include AdaptiveFilter, BilateralFilter, ConservativeSmoothingFilter, 
  DiffOfGaussianFilter, EdgePreservingMeanFilter, EmbossFilter, GaussianFilter, 
  HighPassFilter, KNearestMeanFilter, LaplacianFilter, LaplacianOfGaussianFilter, 
  LeeFilter, MaximumFilter, MeanFilter, MedianFilter, MinimumFilter, OlympicFilter, 
  PrewittFilter, RangeFilter, RobertsCrossFilter, ScharrFilter, SobelFilter, and
  UserDefinedWeightsFilter.
- Fixed a bug with reading/writing Whitebox Raster files containing RGB data.
- Modified the MajorityFilter tool to improve efficiency substantially. Also fixed
  a bug in it and the DiversityFilter tools.


Version 0.7.0 (01-05-2018)
- Added the following tools:
    AttributeCorrelation
    ChangeVectorAnalysis
    ClassifyOverlapPoints
    ClipLidarToPolygon
    ClipRasterToPolygon
    CorrectVignetting
    ErasePolygonFromLidar
    ExportTableToCsv
    RaiseWalls
    TrendSurface
    TrendSurfaceVectorPoints
    UnnestBasins
    UserDefinedWeightsFilter
- Updated TraceDownslopeFlowpaths to take vector seed point inputs.

Version 0.6.0 (22-04-2018)
- Added the ability to read Shapefile attribute data (.dbf files).
- Added support to read LZW compressed GeoTIFFs, at least for simple
  single-band files. The decoder can also handle the use of a horizontal
  predictor (TIFF Tag 317).
- The following tools have been added in this release:
    AttributeHistogram
    AttributeScattergram
    CountIf
    ListUniqueValues
    VectorLinesToRaster
    VectorPointsToRaster
    VectorPolygonsToRaster

Version 0.5.1 (11-04-2018)
- This minor-point release fixes a far-reaching regression bug caused by a 
  change to the Raster class in the previous release. The change was 
  needed for the in-place operator tools added in the last update. This
  modification however affected the proper running of several other tools
  in the library, particularly those in the Math and Stats toolbox. The
  issue has now been resolved. 
- The VisibilityIndex tool has been added to the library. This is one 
  of the most computationally intensive tools in the library and should
  really only be used in a high 
- Modified tools with integer parameter inputs to parse strings 
  representations of floating point numbers. Previously, feeding
  a tool 'filter_size=3.0' would cause a fatal error.
- Changed Raster so that when a filename with no extension is provided
  as a parameter input to a tool, it defaults to a GeoTIFF.
- Added a new section to the User Manual titled, 'An Example WhiteboxTools 
  Python Project'. This addition provides a demonstration of how to
  set-up a WhiteboxTools Python project, including the file structure
  needed to import the library.

Version 0.5.0 (04-04-2018)

- The following tools have been added:
    EdgePreservingMeanFilter
    ElevationAboveStreamEuclidean
    ErasePolygonFromRaster
    FillBurn
    FlattenLakes
    ImageStackProfile
    InPlaceAdd
    InPlaceDivide
    InPlaceMultiply
    InPlaceSubtract
    MaxAnisotropyDevSignature
    PrincipalComponentAnalysis
    RasterizeStreams
    
- Updated tools so that the reporting of elapsed time respects verbose mode.
- Raster now allows for opening an existing raster in write mode ('w'), needed 
  for the in-place math operators (e.g. InPlaceAdd).
- Added update_display_min_max function to Raster.
- Output tables now highlight rows when the mouse hovers.

Version 0.4.0 (04-03-2018)

- This release has erognomic improvements for Python scripting with Whitebox. Tools can be called 
  in Python like this:

  wt = WhiteboxTools()
  wt.slope(‘DEM.dep’, ‘slope.dep’)
  
- There is a convenience method in whitebox_tools.py for each tool in the WhiteboxTools binary 
  executable. This makes it far easier to call tools from a Python script. See the User Manual
  for details.
- Significant improvements and revisions have been made to the User Manual, and in particular 
  the section on Python based scripting with WhiteboxTools.
- The following tools have been added to the library:
    LidarColourize
    LidarPointStats
    LidarRemoveDuplicates
    LongProfile
    LongProfileFromPoints
    MaxElevDevSignature
    MultiscaleRoughness
    MultiscaleRoughnessSignature
    PrintGeoTiffTags
    Profile
- Updated Watershed and Viewshed tools to take vector point inputs.
- PennockLandformClass tool modified to have int8 output data type. Also fixed a bug in the input
  parameters.

Version 0.3.1 (15-02-2018)

- No new tools have been added to this release. Instead the focus was on improving and enhancing
  LAS file support and fixing a numbe of bugs. These include the following:
- Support has been added in the LAS file reader for handling Point Record Formats 4-11 in the 
  LAS 1.4 specificiations. This includes enhanced support for 64-bit LAS files. This change 
  resulted in cascading changes throughout the LiDAR infrastructure and LiDAR tools. Future 
  work will focus on writing LAS files in 1.4 format, instead of the current 1.3 format that is
  saved.
- The LidarIdwInterpolation, LidarNearestNeighbourGridding, and LidarPointDensity tools have each
  been modified to enhance the granularity of parallelism when operating in multi-file mode. This 
  has resulted in large improvements in performance when interpolating entire directories of 
  LAS files.
- The LasHeader object now has the ability to read a LAS header directly. This allows 
  interrogation of a LAS file without the need to create a full LAS object. This was useful 
  for identifying neighbouring tiles during interpolation, such that a buffer of points 
  from adjacent tiles can be used, thereby minimizing interpolation edge effects.
- There was a bug with the WhiteboxTools Runner that had issue with the use of a forward-slash (/) 
  in file paths on Windows. These have been fixed now. I also fixed every tool such that the use
  of a forward slash for file paths on Windows won't result in an additional working directory 
  being appended to file names. This one resulted in many files being slightly modified.
- Added the ability to select the working directory in WhiteboxTools Runner. This is a useful
  feature because some of the LiDAR tools now allow for no specified input/output files, in 
  which case they operate on all of the LAS files contained within the working directory.

Version 0.3 (07-02-2018)

- Added the following tools:
    MaxAnisotropyDev
    HysometricAnalysis
    SlopeVsElevationPlot
    LidarRemoveOutliers

- Added initial support for reading Shapefile geometries. This is still a proof-of-concept
  and no tools as of yet use Shapefile inputs. 
- Added functionality to create beautiful and interactive line graph and scattergram 
  outputs for tools.
- LiDAR interpolation tools now have the option to interpolate all LAS files within the 
  working directory when an input file name is not specified.
- Added first draft of a pdf user manual for WhiteboxTools.

Version 0.2 (12-01-2018)

- Added the following tools:
    KSTestForNormality
    RadomSample
    Mosaic
    Resample
    RadiusOfGyration
    KMeansClustering
    ModifiedKMeansClustering
    D8MassFlux
    DInfMassFlux
    RasterHistogram
    LidarHistogram
    CrossTabulation
    ImageAutocorrelation
    ExtractRasterStatistics
    AggregateRaster
    Viewshed

- Fixed several bugs including one affecting the reading of LAS files.

- Numerous enhancements