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
-r, --run        Runs a tool; used in conjunction with --wd flag; -r="LidarInfo".
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

Or, if Python 3 is the default Python interpreter:

python wb_runner.py

At the command prompt (after cd'ing to this folder, which contains the script).

WhiteboxTools is distributed under a permissive MIT open-source license. See LICENSE.txt 
for more details.

******************
* Release Notes: *
******************

Version 2.2.0 (23-10-2022)
- Added the TravellingSalesmanProblem tool for identifying short routes connecting multiple locations.
- Added the HeatMap tool for performing kernel density estimation (KDE) from vector points.
- Added the MultiplyOverlay tool.
- Added the MaxUpslopeValue tool.
- Added the ConditionedLatinHypercube tool for stratified random sampling (credit Dr. Dan Newman).
- Added the HighPassBilateralFilter tool, useful for emphasizing image texture.
- Fixed a bug with the DirectDecorrelationStretch tool.
- Fixed a bug in the automatic install of the Whitebox extensions that affected Windows users.
- Fixed a bug with the persistence of the compress_rasters parameter. Python users were unable to
  turn off the compress flag previously.
- Added the option to set and get the maximum number of processors (--max_procs flag) used by WBT in 
  the Whitebox Python API.
- Added the option to output average point density and nominal point spacing to the LidarInfo tool.
- Updated the ClassifyOverlapPoints and FlightlineOverlap tools to use information contained within
  the Point Source ID property, rather than a hard-coded time difference threshold previously used.
- Fixed an issue that affected many tools when input rasters use either NaN or Inf as NoData values.
- Fixed an issue with the way that NoData values are handled during the euclidean distance transform
  that impacted the EuclideanDistance, EuclideanAllocation, BufferRaster, and 
  ElevationAboveStreamEuclidean tools.
- Fixed a bug with the LidarInfo tool that occurred when the user did not specify the mandatory 
  output parameter along with a non LAS input file.
- Fixed a bug with the Truncate tool; the output image was always integer, and therefore it did not
  work as expected when using more than zero significant digits.
- Fixed a bug with the ConstructVectorTIN tool that resulted in an error when no field data are used.
- Modified the code for writing to the settings.json file so that rather than issuing an error when
  the app doesn't have write permission, it simply prints a warning and carries on.
- Fixed bugs in the Geomorphons tool (submitted by Dr. Dan Newman).
- Fixed a bug with the writing of PolyLineZ vectors.
- Updated the Hillshade, MultidirectionalHillshade, and RelativeAspect tools to use the more robust 
  5x5 3rd order bivariate polynomial method of Florinsky (2016) for rectangular grid DEMs, and the 
  3x3 method, also described by Florinsky (2016), for DEMs in geographic coordinates. This is a large 
  improvement in accuracy for calculating these surface morphology parameters on geographic coordinates
  compared with the 'z-conversion fudge factor' method used previously.
- Added support for Apple Silicon; you can now download WhiteboxTools binaries compiled on an M1 Mac.

Version 2.1.0 (30-01-2022)
- The Geomorphons tool for landform classification is now available.
- Added the MeanCurvature, GaussianCurvature, MinimalCurvature and MaximalCurvature tools.
- Added GaussianScaleSpace tool, which uses the fast Gaussian approximation algorithm to produce 
  scaled land-surface parameter measurements from an input DEM.
- Added LocalQuadraticRegression tool, which is an implementation of the constrained quadratic 
  regression algorithm using a flexible window size described in Wood (1996).
- Added the MaxUpslopeElevChange tool, the upslope equivalent of the MaxDownslopeElevChange tool.
- Updated the Slope, Aspect, ProfileCurvature, TangentialCurvature, PlanCurvature, and
  TotalCurvature tools to use the more robust 5x5 3rd order bivariate polynomial method
  of Florinsky (2016) for rectangular grid DEMs, and the 3x3 method, also described by
  Florinsky (2016), for DEMs in geographic coordinates. This is a large improvement in
  accuracy for calculating these surface morphology parameters on geographic coordinates
  compared with the 'z-conversion fudge factor' method used previously.
- Added the LidarShift tool for applying a simple shift to point x,y,z coordinates.
- Added the ability to automatically install the Whitebox extensions using the Python API.
- Fixed a bug in the lower quartile valley mapping method of the ExtractValleys tool.
- Fixed a bug in the PennockLandformClass tool.
- Fixed a bug in the Shapefile reader that affected files of the PointZ ShapeType.
- Fixed a bug with the CsvPointsToVector tool.
- Reduced the peak memory usage of the D8Pointer and Rho8Pointer tools by about 37.5%.
- Several other minor bugs have been fixed.

Version 2.0.0 (30-08-2021)
- The most important feature in this release is the addition of support for reading and writing the LAZ
  compressed LiDAR format for all of the LiDAR tools in WBT.
- Added the RasterCalculator tool for performing complex mathematical operations on input rasters.
- Added the ConditionalEvaluation tool for performing an if-then-else operation on an input raster.
- Added the EdgeContamination tool to identify grid cells for which the upslope area extends beyond
  the data extent.
- Added the ExposureTowardsWind tool.
- Added the QuinnFlowAccumulation tool to perform a Quinn et al. (1995) flow accumulation operation.
- Added the QinFlowAccumulation tool to perform a Qin et al. (2007) flow accumulation operation.
- Added the Rho8FlowAccumulation tool to perform a Fairfield and Leymarie (1991) flow accumulation 
  operation.
- LidarHistogram now allows a GPS time parameter, which can be useful for determining the number of
  flightlines in a LiDAR tile.
- Fixed a bug with the Resample tool that created an artifact when reampling to resolutions less than 
  one meter.
- Fixed a bug that prevented plugin tools from being discovered by the open-core when run from the command
  line on PATH when the working directory was something other than WBT.
- Fixed several bugs in the ContoursFromPoints tool.
- Fixed the z_factor calculation for large-extent DEMs in geographic coordinates for the geomorphometric
  shape metric tools, e.g. slope, aspect, hillshade, and curvatures. The new approach calculates a different
  z_factor conversion value for each row in the raster, rather than using a single value based on the raster 
  mid-point latitude. This should help improve the accuracy of these shape indices on large-extent rasters 
  in geographic coordinate systems.
- Fixed several bugs in the isobasins and D8 flow accumulation tool.
- The NewRasterFromBase tool now accepts a vector base file (and grid cell size) as well as a raster.
- The WhiteboxTools user manual has had a refresh and is now hosted at: 
  https://www.whiteboxgeo.com/manual/wbt_book/intro.html
- We have added numerous tools to the WhiteboxTools Extensions. For details see:
  https://www.whiteboxgeo.com/whitebox-geospatial-extensions/

Version 1.5.0 (31-05-2021)
- This release does not include very many new tools. Despite this, this is probably one of the largest 
  releases yet. We have made extensive changes to the codebase, improving functionality in many 
  significant ways. Therefore, we're very excited to announce the release of v1.5.
- Probably the most exciting change is the introduction of plugin tools. Up until now, WBT has had a 
  monolithic architecture, where all of the tools live within a single binary. This architecture has 
  provided a number of benefits up until now. However, as the number of tools in WBT grows, it becomes
  increasingly difficult to maintain this program structure - in particular, compile times have grown
  significantly since the projects start. A plugin architecture provides much greater flexibility in 
  this regard. Single tool plugins can be created, placed within the new 'plugins' folder within the 
  WBT directory, and the whitebox_tools.exe binary will treat these plugins like any other tool within
  the monolith. This also means that WBT users can develop their own custom tools, without the required 
  know-how of figuring out how to integrate their tool into the large WBT code-base. The user manual
  will be updated shortly to describe how this process works. For now, there is only one plugin tool 
  example in the open-core (SplitVectorLines) although several other plugins have been developed (more 
  on this below). The one downside of the new plugin architecture is that the size of the WBT download
  will inevitably grow, as individual tool executables will be larger than the single monolith. We 
  believe that this is an acceptable compromise. 
- In order to accommodate plugin tools, we have significantly changed the codebase. Most significantly 
  We have pulled the code associated with low-level functions, the so-called 'plumbing' code, (e.g. 
  code for reading and writing various data files) into separate sub-repositories. In this way, the 
  tools in the monolith and the plugin tools can both use this code without duplication.
- WBT now has persistent environment variables contained within a 'settings.json' file within the WBT 
  folder. Currently, these settings including 'verbose_mode', 'working_directory', 'compress_rasters', 
  and 'max_procs'. More environment variables may be added in later releases. The fact that verbose mode
  the working directory, and the compress_rasters flag are now persistent does have implications for the
  Python and R front-ends and for the way these settings are used. The user manual will be updated 
  shortly to reflect these changes.
- We introduced the 'max_procs' setting. Now, all tools that run in parallel, or partially parallelize,
  can be restricted to a maximum number of processes. Before, WBT would simply use all of the available
  cores in the machine it was running on. While this is still the default (`max_procs: -1`), there are
  certain conditions where this behaviour is undesirable. For example, when WBT is run on large servers 
  or cloud-computing environments where a great many cores are shared among many users, it can be 
  problematic when a single user/program consumes all of the available cores. This setting limits 
  the maximum number of procs.
- Added the EmbankmentMapping tool for mapping transportation embankments (road, rail) in LiDAR DEMs.
- Added the SplitVectorLines tool. This tool will parse an input line vector into a series of segments
  of a specified length. It is also an example of a WBT plugin.
- The code has been updated to reflect the new zLidar v1.1 specification, which has significantly improved 
  compression rates (testing shows it is between 91% and 109% of LAZ), greater flexibility (users may
  specify the degree of compression versus speed of reading/writing), and numerous bug fixes. The zLidar 
  specification webpage will soon be updated to reflect the new version. Further news on this front, 
  it has come to our attention recently that there is now a Rust-based LAZ encoder/decoder, which provides 
  an opportunity for us to add LAZ support in a future version of WBT. We are currently evaluating this
  as an option.
- We are trying to be more engaging with the WBT user community. In this regard, we have set up a new
  Google Groups forum for user to ask questions (https://groups.google.com/g/whiteboxtools), and have a 
  new Twitter account (@whiteboxgeo) and newsletter to make announcements. Feel free to sign up for 
  either if you're interested in staying in touch. 
- Lastly, we are very pleased to announce the creation of WhiteboxTools Geospatial Inc., a new company 
  based on providing extension services around the open-source WBT platform. It is our vision that this
  company will provide a way of making the ongoing development of the WBT open-core more sustainable in 
  the future, by enabling developers to work full-time on the project. Please read my 'open letter to the 
  WBT community' (https://www.whiteboxgeo.com/open-letter-whiteboxtools-community/) for more details 
  about this exciting development. Our plan is to maintain, and continue development of, the open-core of 
  WBT, while providing plugin extensions that enhance the core capabilities. To begin with, we are launching
  the Whitebox General Toolset Extension, a set of (currently) 19 tools to help GIS professionals with their
  everyday workflows. Please see the newly redesigned WBT webpage at www.whiteboxgeo.com for more details.
  If you have been interested in supporting the WBT project in the past and haven't known how, buying a 
  license for the General Toolset Extension is a wonderful way of doing so.

Version 1.4.0 (04-09-2020)
- Added the TimeInDaylight model tool for modelling the proportion of daytime that a location is not in shadow.
- Added the MapOffTerrainObjects tool.
- Added the FilterRasterFeaturesByArea tool.
- Added the LidarDigitalSurfaceModel tool.
- The D8 and FD8 flow pointer tools now output byte rasters.
- The Isobasins tool now optionally outputs an upstream/downstream connections table.
- The HorizonAngle tool has had significant performance improvements.
- Improvements to the RemoveOffTerrainObjects tool's performance.
- The Resample tool has been modified so that it does not require a 'destination' raster. Instead,
  it will create a new output raster either based on a user-specified target cell resolution or
  an optional base raster, much like the vector-to-raster conversion tools.
- Tools that input a z_factor conversion no longer override user input with geographic coordinates
  (see issue #113).
- The StreamLinkIdentifier tool now outputs a 32-bit integer format, increasing the maximum allowable 
  number of streams (see issue #110).
- Fixed a bug with cubic-convolution and bilinear resampling in the Mosaic tool (see issue #109).

Version 1.3.1 (23-07-2020)
- Added the HypsometricallyTintedHillshade tool to create hypsometric tinted hillshades.
- Added the MultidirectionalHillshade tool.
- Added the ability to read/write in the Esri BIL raster format.
- Added the LidarRooftopAnalysis tool.
- The MultiPartToSinglePart tool now handles MultiPoint vectors.
- Fixed a bug with the VoronoiDiagram to better handle MultiPoint vectors.
- Fixed an issue with writing compressed RGB GeoTIFFs.
- Fixed an issue reading LZW compressed GeoTIFFs.

Version 1.3.0 (07-06-2020)
- Tools will now output DEFLATE compressed GeoTIFFs when the --compress_rasters parameter is used.
- Added support for a newly developed compressed LiDAR data format, the ZLidar file. All tools
  that accepted LAS file inputs and produced LAS outputs can now use '*.zlidar' files as well. I 
  have also added the LasToZlidar and ZlidarToLas tools to perform conversions. While the ZLidar
  format does not yield compression rates quite as good as the popular LAZ file format, you can  
  expect ZLidar files to be between 20-30% of the size of the equivalent LAS file. A file  
  specification will be published in the near future to describe the open ZLidar data format.
- Added the AsciiToLas tool.
- Added the ContoursFromRaster tool for creating a vector contour coverage from a raster surface model (DEM).
- Added the ContoursFromPoints tool for creating a vector contour coverage from vector points.
- Added the UpdateNodataCells tool.
- Modified the Slope tool to optionally output in degrees, radians, or percent gradient.
- Modified the Mosaic tool, which now runs much faster with large numbers of input tiles.
- The vector-to-raster conversion tools now preserve input projections.
- Fixed a bug in the RasterToVectorPolygons tool.
- Fixed several bugs in the MergeTableWithCsv tool.
- Modified the FillMissingData tool to allow for the exclusion of edge-connected NoData cells from the operation.
  This is better for irregular shaped DEMs that have large areas of NoData surrounding the valid data.
- The LidarConstructVectorTin tool has been depreciated. The tool was not creating the proper output.
  Furthermore, since the number of points in the average LiDAR tile is usually many million, this tool
  would usually produce Shapefiles that exceed the maximum allowable number of shape geometries. If 
  a vector TIN is required for a LiDAR point set, users should convert the file to a Shapefile and then
  use the ConstructVectorTin tool instead. And of course, if you are interested in a raster TIN from a 
  LiDAR file, use the LidarTinGridding tool instead. 
- FlattenLakes now handles multipart lake polygons.

Version 1.2.0 (21-02-2020)
- Added the RasterToVectorPolygons tool, which now completes the raster-vector conversion tool set.
- Added the MultiscaleElevationPercentile tool.
- Added the AttributeCorrelationNeighbourhoodAnalysis tool.
- Added the RadialBasisFunctionInterpolation tool, which includes a thin-plate spline mode.
- Added the RasterPerimeter tool to measure the perimeter of raster polygons.
- Added the MDInfFlowAccumulation tool to perform the MD-infinity flow accumulation of Seibert 
  and McGlynn (2007).
- Added the InsertDams tool, which can be used to insert impoundment features at a set of points
  of interest into a DEM. This tool can be used in combination with the ImpoundmentSizeIndex tool 
  to create artificial reservoirs/depressions.
- Added the HeightAboveGround tool, to normalize a LiDAR point cloud. Each point's z-value is
  converted to the height above the nearest ground-classified point.
- Modified the LidarRbfInterpolation tool to improve efficiency.
- Fixed an issue with how floating point attributes were written in Shapefile attribute tables.
- Updated the LidarSegmentation tool, which now used RANSAC to fit planar models to points.
- Fixed an issue with the Reclass and ReclassFromFile tool that caused striping.
- The Relcass and ReclassFromFile tools now take 'min' and 'max' in the reclass string.
- The watershed tool now accepts either a set of vector points or a raster for the pour points 
  file. If a raster is specified, all non-zero, non-NoData valued cells will be considered 
  outlet cells and the watershed labels will be assigned based on these values.
- The D8 and D-infinity flow accumulation tools now take either an input DEM or a flow pointer raster 
  as inputs.

Version 1.1.0 (09-12-2019)
- Added the BreachDepressionsLeastCost tool, which performs a modified form of the Lindsay 
  and Dhun (2015) impact minimizing breaching algorithm. This modified algorithm is very 
  efficient and can provide an excellent method for creating depressionless DEMs from large 
  DEMs, including those derived from LiDAR. It is particularly well suited to breaching 
  through road embankments, approximately the pathway of culverts.
- The FillDepressions tool algorithm has been completely re-developed. The new algorithm is
  significantly faster than the previous method, which was based on the Wang and Lui method.
  For legacy reasons, the previous tool has been retained and renamed FillDepressonsWangAndLui.
  Notice that this new method also incorporates significantly improved flat area correction
  that maintains general flowpaths of filled areas.
- The Sink and DepthInSink tools have been updated to use the new depression filling algorithm.
- Added the ClassifyBuildingsInLidar tool to reclassify LiDAR points within a LAS file
  to the building class value (6) that are located within one or more building footprint
  contained in an input polygon vector file. 
- Added the NaturalNeighbourInterpolation tool for performing Sibson's (1981) interpolation
  method on input point data.
- Added the UpslopeDepressionStorage tool to estimate the average upslope depression 
  storage capacity (DSC).
- Added the LidarRbfInterpolation tool for performing a radial basis function (RBF) interpolation
  of LiDAR data sets.
- The WhiteboxTools Runner user interface has been significantly improved (many thanks to 
  Rachel Broders for these contributions).
- Fixed a bug in which the photometric interpretation was not being set by certain raster
  decoders, including the SAGA encoder. This was causing an error when outputting GeoTIFF 
  files.
- Updated the ConstructVectorTIN and TINGridding tools to include a maximum triangle edge 
  length to help avoid the creation of spurious long and narrow triangles in convex regions 
  along the data boundaries.
- Added the ImageCorrelationNeighbourhoodAnalysis tool for performing correlation analysis
  between two input rasters within roving search windows. The tool can be used to perform
  Pearson's r, Spearman's Rho, or Kendall's Tau-b correlations.

Version 1.0.2 (01-11-2019)
- Added the BurnStreamsAtRoads tool.
- Added a two-sample K-S test (TwoSampleKsTest) for comparing the distributions of two rasters.
- Added a Wilcoxon Signed-Rank test (WilcoxonSignedRankTest) for comparing two rasters.
- Added a paired-samples Student's t-test (PairedSampleTTest) for comparing two rasters.
- Added the inverse hyperbolic trig functions, i.e. the Arcosh, Arsinh, and Artanh tools.
- Renamed the LeeFilter to the LeeSigmaFilter.
- Renamed the RelativeStreamPowerIndex tool to StreamPowerIndex, to be more in-line with
  other software.
- Fixed another bug related to the handling of Boolean tool parameters.

Version 1.0.1 (20-10-2019)
- Boolean type tool parameters previously worked simply by the presence of the parameter flag.
  This was causing problems with some WBT front-ends, particularly QGIS, where the parameters
  were being provided to WBT as --flag=False. In this case, because the flag was present, it 
  was assumed to be True. All tools that have boolean parameters have been updated to handle
  the case of --flag=False. This is a widespread modification that should fix the unexpected
  behaviour of many tools in certain front-ends.
- Fixed a minor bug with the VectorPolygonToRaster tool.
- Fixed a bug in the DownstreamDistanceToStream tool.

Version 1.0.0 (29-09-2019)
- Added support for reading and writing the BigTIFF format. This has resulted in numerous changes
  throughout the codebase as a result of significant modification of ByteOrderReader and addition
  of ByteOrderWriter. This change has touched almost every one of the raster format 
  encoders/decoders.
- Performance improvements have been made to the FlattenLakes (hydro-flattening) tool.
- Fixed a bug preventing the SplitColourComposite tool from reading the '--input' flag correctly.
- The ClipLidarToPolygon now issues a warning if the output LAS file does not contain any points
  within the clipped region and does not output a file. Also, the LAS reader no longer panics 
  when it encounters a file with no points. Now it reads the header file, issues a warning, and 
  carries on, allowing the tools to handle the case of no points.
- ImageRegression can now optionally output a scatterplot. The scatterplot is based on a random 
  sample of a user-defined size.
- Added the CleanVector tool.
- ExtractRasterStatistics has been renamed ZonalStatistics to be more inline with other GIS, 
  including ArcGIS and QGIS.
- Added the median as a statistic that ZonalStatistics provides.
- Fixed a bug in the VectorPolygonsToRaster tool that sometimes mishandled polygon holes.
- Added the FilterLidarClasses tool to filter out points of user-specified classes.
- Added the LidarRansacPlanes tool to identify points belonging to planar surfaces. This tool
  uses the RANSAC method, which is a robust modelling method that handles the presence of 
  numerous outlier points.
- The ClipLidarToPolygon tool has been parallelized.
- The LasToAscii and AsciiToLas tools have been updated to handle RGB colour data for points.
- Added the CsvPointsToVector tool to convert a CSV text table into a shapefile of vector points. 
  The table must contain x and y coordinate fields.
- The FeaturePreservingDenoise was renamed to FeaturePreservingSmoothing. The DrainagePreservingSmoothing
  tool was removed. Use FeaturePreservingSmoothing instead.
- Added the ability to output the average number of point returns per pulse in the LidarPointStats tool.
- LidarTinGridding, LidarIdwIntarpolation, and LidarNearestNeighbourGridding now can interpolate the 
  return number, number of returns, and RGB colour data associated with points in a LAS file.
- Added the ModifyNoDataValue tool to change the NoData value in a raster. It updates the value in 
  the raster header and then modifies each grid cell in the raster containing the old NoData value
  to the new value. This operation overwrites the existing raster.
- Fixed an issue with GeoTIFF NoData values that impacted many tools. NoData values were not interpreted
  correctly when they were very large positive or negative values (near the min/max of an f32).

Version 0.16.0 (24-05-2019)
- Added the MergeLineSegments and SphericalStdDevOfNormals tools.
- Fixed a bug with reading LAS files with point records with extra bytes. Previously, the LAS decoder
  assumed the Point Record Length matched that of the LAS specifications (with the variable of the 
  optional intensity and user data). Some LAS files in the wild (particularly those created using 
  LASTools and of LAS version 1.2) have larger Point Record Lengths, which presumably carry extra 
  bytes of information. These extra byes are ignored, but they no longer throw off the decoding.
- Fixed a bug with writing Big-Ending GeoTIFF files. The 'MM' file header was not correct previously.
- Significantly reduced the memory requirements of the StochasticDepressionAnalysis tool. The tool 
  may be somewhat slower as a result, but it should be applicable to larger DEMs than was previously
  possible.
- Fixed bugs with the Union and SplitWithLines tools. 
- WhiteboxTools can now read and write Shapefiles of MultiPointZ, PolyLineZ, and PolygonZ ShapeTypes 
  missing the optional 'M' values (i.e. measures).
- SelectTilesByPolygon and LidarTileFootprint are now compatible with LAZ file inputs. Both of these 
  tools only rely on information in the input LiDAR file's header, which is the same for a LAZ file 
  as a LAS file.
- Fixed a bug with writing Saga GIS files (*.sdat) that inverted rasters.

Version 0.15.0 (03-03-2019)
- The following tools were added to the project:
  BoundaryShapeComplexity
  NarrownessIndex
  ShapeComplexityIndexRaster

- Fixed a bug with the PanchromaticSharpening tool.
- Previously, if a LAS v1.4 file were input to a tool, the output LAS file, which is currently
  always in LAS v1.3 format, would not correctly translate the 64-bit information (point 
  return, number of returns, classification) into 32-bit format. I have added the 
  get_32bit_from_64bit function to handle this translation more gracefully; albeit it is
  still a lossy translation where returns greater than 5 are ignored and classification 
  values greater than 31 are lost. 
- Added a maximum triangle edge length parameter to the LidarTinGridding tool to allow 
  for the exclusion of large-area triangles (i.e. low point density) from the gridding.
- The NormalizedDifferenceVegetationIndex tool has been renamed to NormalizedDifferenceIndex 
  to indicate the more general nature of this tool (i.e. NDVI, NDWI, OSAVI, etc.).
- Significant changes have been made to the BreachDepressions tool to make it more in-line
  with the behaviour of the GoSpatial algorithm described in the original Lindsay (2016)
  paper. These changes include: 1) the inclusion of an optional parameter to fill single-cell
  pits prior to breaching, 2) the addition of a --flat_increment parameter, which overrides
  the automatically derived value assigned to flat areas along breach channels (or filled 
  depressions), and 3) the tool now performs a fast post-breach filling operation, when
  run in constrained-breaching mode (i.e. when the user specifies values for either 
  --max_depth or --max_length, placing constraints on the allowable breach channel size).

Version 0.14.1 (10-02-2019)
- This release largely focuses on bug-fixes rather than feature additions. However, the
  following tools were added to the library:
  RasterArea
  
- Fixed a bug with the MultiscaleTopographicPositionImage tool that prevented proper output
  for files in GeoTIFF format. 
- Several other tool-specific bug fixes.

Version 0.14.0 (27-01-2019)
- The release largely focuses on bug-fixes rather than adding new features. The
 following tools were added to the project:
    CircularVarianceOfAspect
    EdgeDensity
    SurfaceAreaRatio

- Fixed a bug that resulted in rasters with projected coordinate systems being
  interpreted as geographic coordinates, thereby messing up the calculation of 
  inter-cell distances for tools like slope, aspect, curvature, etc.
- Fixed a bug with several of the math tools; output files took their data type
  from the input file. In some cases, this does not work well because the input
  is integer and the output must be floating point data.


Version 0.13.0 (08-01-2019)
- The release largely focuses on bug-fixes rather than adding new features. The
 following tools were added to the project:
    MosaicWithFeathering
- Support was added for GeoTIFF MODELTRANSFORMATIONTAG (Tag 33920).
- Support was added for reading GeoTIFFs that have coordinate transformations 
  defined by multiple tiepoints contained with the ModelTiepointTag (Tag 33922).
  These rasters have their raster-to-model transform defined by a 2D polynomial
  regression of the 3rd order.
- The initialize_using_file function in the abstract Raster model now transfers
  information contained in an input GeoTIFF's ModelTiePoint, ModelPixelScale,
  ModelTransform, GeoKeyDirectory, GeoDoubleParms, and GeoAsciiParams tags to
  the output raster. This means that if a GeoTIFF file is input to a Whitebox 
  tool, and the output raster is specified to be of GeoTIFF format as well,
  all of the coordinate information contain in the input raster will now be
  contained in the output raster.
- The FeaturePreservingDenoise and DrainagePreservingSmoothing tools, both of
  which are used for DEM generalization, now represent surface normal vectors 
  using 32-bit floats instead of the original double-precision values. This 
  does not alter the results of these tools significantly, but does reduce the 
  memory requirements and run-times of these tools substantially.
- The LidarKappa tool now outputs a raster displaying the spatial distribution 
  of the overall accuracy per grid cell (i.e. percent agreement).
- Fixed a bug with the RasterStreamsToVector tool that resulted in overlapping
  traced streams.
- The D8FlowAccumulation tool has been modified to use a fixed flow-width to 
  calculate specific contributing area, equal to the average grid cell resolution. 
  The tool previously used a variable flow-width for SCA calculations, however,
  1. this differs from the constant value used in Whitebox GAT, and 2. a 
  variable flow-width means that flow accumulation values do not increase 
  continuously in a downstream direction. This last issue was causing problems
  with applications involving stream network extraction. This change does not
  affect the 'cells' nor 'catchment area' outputs of the tool.
- Fixed a bug with the GeoTIFF NoData tag.
- Fixed a bug with the SetNodataValue tool.


Version 0.12.0 (22-11-2018)
- The following tools were added to the project:
    BlockMaximumGridding
    BlockMinimumGridding
    Clip
    Dissolve
    Erase
    JoinTables
    Intersect
    LasToShapefile
    LidarClassifySubset
    LinearityIndex
    LineIntersections
    LongestFlowpath
    MergeTableWithCsv
    MergeVectors
    NearestNeighbourGridding
    PatchOrientation
    Polygonize
    RasterToVectorLines
    SplitWithLines
    SymmetricalDifference
    Union
    VoronoiDiagram

- Modified the algorithm used by the CostDistance tool from an iterative method of
  finding the minimum cost surface to one that uses a priority-flood approach. This
  is far more efficient. Also, there was a bug in the original code that was the 
  result of a mismatch between the neighbouring cell distances and the back-link 
  direction. In some cases this resulted in an infinite loop, which is now resolved.
- Improvements have been made to the WhiteboxTools GeoTIFF reader. A bug has been
  fixed that prevented tile-oriented (in contrast to the more common strip-oriented)
  TIFF files from being read properly. Support has been added for reading rasters
  that have been compressed using the DEFLATE algorithm. Lastly, the WhiteboxTools
  GeoTIFF reader now supports sparse rasters, as implemented by GDAL's GeoTIFF driver.
- An issue in the SAGA raster format reader has been fixed.


Version 0.11.0 (01-10-2018)
- This release is marked by the addition of several vector data processing capabilities. 
  Most notably, this includes support for TINing and TIN based gridding (vector and 
  LiDAR), as well as several vector patch shape indices. The following tools were 
  added to the project:
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
  LAS 1.4 specifications. This includes enhanced support for 64-bit LAS files. This change 
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