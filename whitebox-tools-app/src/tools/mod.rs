pub mod data_tools;
pub mod gis_analysis;
pub mod hydro_analysis;
pub mod image_analysis;
pub mod lidar_analysis;
pub mod math_stat_analysis;
pub mod stream_network_analysis;
pub mod terrain_analysis;

use whitebox_common::utils::get_formatted_elapsed_time;
use serde_json;
use std::io::{Error, ErrorKind};
use std::time::Instant;
use std::path;
use std::fs;
use std::collections::HashMap;
use std::process::Command;
use std::env;
// use std::io;
// use std::path::PathBuf;

#[derive(Default)]
pub struct ToolManager {
    pub working_dir: String,
    pub verbose: bool,
    tool_names: Vec<String>,
}

impl ToolManager {
    pub fn new<'a>(
        working_directory: &'a str,
        verbose_mode: &'a bool,
    ) -> Result<ToolManager, Error> {
        let mut tool_names = vec![];
        // data_tools
        tool_names.push("AddPointCoordinatesToTable".to_string());
        tool_names.push("CleanVector".to_string());
        tool_names.push("ConvertNodataToZero".to_string());
        tool_names.push("ConvertRasterFormat".to_string());
        tool_names.push("CsvPointsToVector".to_string());
        tool_names.push("ExportTableToCsv".to_string());
        tool_names.push("JoinTables".to_string());
        tool_names.push("LinesToPolygons".to_string());
        tool_names.push("MergeTableWithCsv".to_string());
        tool_names.push("MergeVectors".to_string());
        tool_names.push("ModifyNoDataValue".to_string());
        tool_names.push("MultiPartToSinglePart".to_string());
        tool_names.push("NewRasterFromBase".to_string());
        tool_names.push("PolygonsToLines".to_string());
        tool_names.push("PrintGeoTiffTags".to_string());
        tool_names.push("RasterToVectorLines".to_string());
        tool_names.push("RasterToVectorPoints".to_string());
        tool_names.push("RasterToVectorPolygons".to_string());
        tool_names.push("ReinitializeAttributeTable".to_string());
        tool_names.push("RemovePolygonHoles".to_string());
        tool_names.push("SetNodataValue".to_string());
        tool_names.push("SinglePartToMultiPart".to_string());
        tool_names.push("VectorLinesToRaster".to_string());
        tool_names.push("VectorPointsToRaster".to_string());
        tool_names.push("VectorPolygonsToRaster".to_string());

        // gis_analysis
        tool_names.push("AggregateRaster".to_string());
        tool_names.push("AverageOverlay".to_string());
        tool_names.push("BlockMaximumGridding".to_string());
        tool_names.push("BlockMinimumGridding".to_string());
        tool_names.push("BoundaryShapeComplexity".to_string());
        tool_names.push("BufferRaster".to_string());
        // tool_names.push("BufferVector".to_string());
        tool_names.push("Centroid".to_string());
        tool_names.push("CentroidVector".to_string());
        tool_names.push("Clip".to_string());
        tool_names.push("ClipRasterToPolygon".to_string());
        tool_names.push("Clump".to_string());
        tool_names.push("CompactnessRatio".to_string());
        tool_names.push("ConstructVectorTIN".to_string());
        tool_names.push("CountIf".to_string());
        tool_names.push("CostAllocation".to_string());
        tool_names.push("CostDistance".to_string());
        tool_names.push("CostPathway".to_string());
        tool_names.push("CreateHexagonalVectorGrid".to_string());
        tool_names.push("CreatePlane".to_string());
        tool_names.push("CreateRectangularVectorGrid".to_string());
        tool_names.push("Difference".to_string());
        tool_names.push("Dissolve".to_string());
        tool_names.push("EdgeProportion".to_string());
        tool_names.push("EliminateCoincidentPoints".to_string());
        tool_names.push("ElongationRatio".to_string());
        tool_names.push("Erase".to_string());
        tool_names.push("ErasePolygonFromRaster".to_string());
        tool_names.push("EuclideanAllocation".to_string());
        tool_names.push("EuclideanDistance".to_string());
        tool_names.push("ExtendVectorLines".to_string());
        tool_names.push("ExtractNodes".to_string());
        tool_names.push("ExtractRasterValuesAtPoints".to_string());
        tool_names.push("FilterRasterFeaturesByArea".to_string());
        tool_names.push("FindLowestOrHighestPoints".to_string());
        tool_names.push("FindPatchOrClassEdgeCells".to_string());
        tool_names.push("HighestPosition".to_string());
        tool_names.push("HoleProportion".to_string());
        tool_names.push("IdwInterpolation".to_string());
        tool_names.push("Intersect".to_string());
        tool_names.push("LayerFootprint".to_string());
        tool_names.push("LinearityIndex".to_string());
        tool_names.push("LineIntersections".to_string());
        tool_names.push("LowestPosition".to_string());
        tool_names.push("MaxAbsoluteOverlay".to_string());
        tool_names.push("MaxOverlay".to_string());
        tool_names.push("Medoid".to_string());
        tool_names.push("MergeLineSegments".to_string());
        tool_names.push("MinAbsoluteOverlay".to_string());
        tool_names.push("MinimumBoundingBox".to_string());
        tool_names.push("MinimumBoundingCircle".to_string());
        tool_names.push("MinimumBoundingEnvelope".to_string());
        tool_names.push("MinimumConvexHull".to_string());
        tool_names.push("MultiplyOverlay".to_string());
        tool_names.push("NarrownessIndex".to_string());
        tool_names.push("NaturalNeighbourInterpolation".to_string());
        tool_names.push("NearestNeighbourGridding".to_string());
        tool_names.push("MinOverlay".to_string());
        tool_names.push("PatchOrientation".to_string());
        tool_names.push("PercentEqualTo".to_string());
        tool_names.push("PercentGreaterThan".to_string());
        tool_names.push("PercentLessThan".to_string());
        tool_names.push("PerimeterAreaRatio".to_string());
        tool_names.push("PickFromList".to_string());
        tool_names.push("PolygonArea".to_string());
        tool_names.push("PolygonLongAxis".to_string());
        tool_names.push("PolygonPerimeter".to_string());
        tool_names.push("PolygonShortAxis".to_string());
        tool_names.push("Polygonize".to_string());
        tool_names.push("RadialBasisFunctionInterpolation".to_string());
        tool_names.push("RadiusOfGyration".to_string());
        tool_names.push("RasterArea".to_string());
        tool_names.push("RasterCellAssignment".to_string());
        tool_names.push("RasterPerimeter".to_string());
        tool_names.push("Reclass".to_string());
        tool_names.push("ReclassEqualInterval".to_string());
        tool_names.push("ReclassFromFile".to_string());
        tool_names.push("RelatedCircumscribingCircle".to_string());
        tool_names.push("ShapeComplexityIndex".to_string());
        tool_names.push("ShapeComplexityIndexRaster".to_string());
        tool_names.push("SmoothVectors".to_string());
        tool_names.push("SplitWithLines".to_string());
        tool_names.push("SumOverlay".to_string());
        tool_names.push("SymmetricalDifference".to_string());
        tool_names.push("TINGridding".to_string());
        tool_names.push("Union".to_string());
        tool_names.push("UpdateNodataCells".to_string());
        tool_names.push("VectorHexBinning".to_string());
        tool_names.push("VoronoiDiagram".to_string());
        tool_names.push("WeightedOverlay".to_string());
        tool_names.push("WeightedSum".to_string());

        // hydro_analysis
        tool_names.push("AverageFlowpathSlope".to_string());
        tool_names.push("AverageUpslopeFlowpathLength".to_string());
        tool_names.push("Basins".to_string());
        tool_names.push("BreachDepressions".to_string());
        tool_names.push("BreachDepressionsLeastCost".to_string());
        tool_names.push("BreachSingleCellPits".to_string());
        tool_names.push("BurnStreamsAtRoads".to_string());
        tool_names.push("D8FlowAccumulation".to_string());
        tool_names.push("D8MassFlux".to_string());
        tool_names.push("D8Pointer".to_string());
        tool_names.push("DepthInSink".to_string());
        tool_names.push("DInfFlowAccumulation".to_string());
        tool_names.push("DInfMassFlux".to_string());
        tool_names.push("DInfPointer".to_string());
        tool_names.push("DownslopeDistanceToStream".to_string());
        tool_names.push("DownslopeFlowpathLength".to_string());
        tool_names.push("ElevationAboveStream".to_string());
        tool_names.push("ElevationAboveStreamEuclidean".to_string());
        tool_names.push("FD8FlowAccumulation".to_string());
        tool_names.push("FD8Pointer".to_string());
        tool_names.push("FillBurn".to_string());
        tool_names.push("FillDepressions".to_string());
        tool_names.push("FillDepressionsPlanchonAndDarboux".to_string());
        tool_names.push("FillDepressionsWangAndLiu".to_string());
        tool_names.push("FillSingleCellPits".to_string());
        tool_names.push("FindNoFlowCells".to_string());
        tool_names.push("FindParallelFlow".to_string());
        tool_names.push("FlattenLakes".to_string());
        tool_names.push("FloodOrder".to_string());
        tool_names.push("FlowAccumulationFullWorkflow".to_string());
        tool_names.push("FlowLengthDiff".to_string());
        tool_names.push("Hillslopes".to_string());
        tool_names.push("ImpoundmentSizeIndex".to_string());
        tool_names.push("InsertDams".to_string());
        tool_names.push("Isobasins".to_string());
        tool_names.push("JensonSnapPourPoints".to_string());
        tool_names.push("LongestFlowpath".to_string());
        tool_names.push("MaxUpslopeFlowpathLength".to_string());
        tool_names.push("MDInfFlowAccumulation".to_string());
        tool_names.push("NumInflowingNeighbours".to_string());
        tool_names.push("RaiseWalls".to_string());
        tool_names.push("Rho8Pointer".to_string());
        tool_names.push("Sink".to_string());
        tool_names.push("SnapPourPoints".to_string());
        tool_names.push("StochasticDepressionAnalysis".to_string());
        tool_names.push("StrahlerOrderBasins".to_string());
        tool_names.push("Subbasins".to_string());
        tool_names.push("TraceDownslopeFlowpaths".to_string());
        tool_names.push("UnnestBasins".to_string());
        tool_names.push("UpslopeDepressionStorage".to_string());
        tool_names.push("Watershed".to_string());

        // image_analysis
        tool_names.push("AdaptiveFilter".to_string());
        tool_names.push("BalanceContrastEnhancement".to_string());
        tool_names.push("BilateralFilter".to_string());
        tool_names.push("ChangeVectorAnalysis".to_string());
        tool_names.push("Closing".to_string());
        tool_names.push("ConservativeSmoothingFilter".to_string());
        tool_names.push("CornerDetection".to_string());
        tool_names.push("CorrectVignetting".to_string());
        tool_names.push("CreateColourComposite".to_string());
        tool_names.push("DirectDecorrelationStretch".to_string());
        tool_names.push("DiversityFilter".to_string());
        tool_names.push("DiffOfGaussianFilter".to_string());
        tool_names.push("EdgePreservingMeanFilter".to_string());
        tool_names.push("EmbossFilter".to_string());
        tool_names.push("FastAlmostGaussianFilter".to_string());
        tool_names.push("FlipImage".to_string());
        tool_names.push("GammaCorrection".to_string());
        tool_names.push("GaussianContrastStretch".to_string());
        tool_names.push("GaussianFilter".to_string());
        tool_names.push("HighPassFilter".to_string());
        tool_names.push("HighPassMedianFilter".to_string());
        tool_names.push("HistogramEqualization".to_string());
        tool_names.push("HistogramMatching".to_string());
        tool_names.push("HistogramMatchingTwoImages".to_string());
        tool_names.push("IhsToRgb".to_string());
        tool_names.push("ImageStackProfile".to_string());
        tool_names.push("IntegralImage".to_string());
        tool_names.push("KMeansClustering".to_string());
        tool_names.push("KNearestMeanFilter".to_string());
        tool_names.push("LaplacianFilter".to_string());
        tool_names.push("LaplacianOfGaussianFilter".to_string());
        tool_names.push("LeeSigmaFilter".to_string());
        tool_names.push("LineDetectionFilter".to_string());
        tool_names.push("LineThinning".to_string());
        tool_names.push("MajorityFilter".to_string());
        tool_names.push("MaximumFilter".to_string());
        tool_names.push("MeanFilter".to_string());
        tool_names.push("MedianFilter".to_string());
        tool_names.push("MinMaxContrastStretch".to_string());
        tool_names.push("MinimumFilter".to_string());
        tool_names.push("ModifiedKMeansClustering".to_string());
        tool_names.push("Mosaic".to_string());
        tool_names.push("MosaicWithFeathering".to_string());
        tool_names.push("NormalizedDifferenceIndex".to_string());
        tool_names.push("OlympicFilter".to_string());
        tool_names.push("Opening".to_string());
        tool_names.push("PanchromaticSharpening".to_string());
        tool_names.push("PercentageContrastStretch".to_string());
        tool_names.push("PercentileFilter".to_string());
        tool_names.push("PrewittFilter".to_string());
        tool_names.push("RangeFilter".to_string());
        tool_names.push("RemoveSpurs".to_string());
        tool_names.push("Resample".to_string());
        tool_names.push("RgbToIhs".to_string());
        tool_names.push("RobertsCrossFilter".to_string());
        tool_names.push("ScharrFilter".to_string());
        tool_names.push("SigmoidalContrastStretch".to_string());
        tool_names.push("SobelFilter".to_string());
        tool_names.push("SplitColourComposite".to_string());
        tool_names.push("StandardDeviationContrastStretch".to_string());
        tool_names.push("StandardDeviationFilter".to_string());
        tool_names.push("ThickenRasterLine".to_string());
        tool_names.push("TophatTransform".to_string());
        tool_names.push("TotalFilter".to_string());
        tool_names.push("UnsharpMasking".to_string());
        tool_names.push("UserDefinedWeightsFilter".to_string());
        tool_names.push("WriteFunctionMemoryInsertion".to_string());

        // lidar_analysis
        tool_names.push("AsciiToLas".to_string());
        tool_names.push("LidarBlockMaximum".to_string());
        tool_names.push("LidarBlockMinimum".to_string());
        tool_names.push("ClassifyBuildingsInLidar".to_string());
        tool_names.push("ClassifyOverlapPoints".to_string());
        tool_names.push("ClipLidarToPolygon".to_string());
        // tool_names.push("ContourLidar".to_string());
        tool_names.push("ErasePolygonFromLidar".to_string());
        tool_names.push("FilterLidarClasses".to_string());
        tool_names.push("FilterLidarScanAngles".to_string());
        tool_names.push("FindFlightlineEdgePoints".to_string());
        tool_names.push("FlightlineOverlap".to_string());
        tool_names.push("HeightAboveGround".to_string());
        tool_names.push("LasToAscii".to_string());
        tool_names.push("LasToMultipointShapefile".to_string());
        tool_names.push("LasToShapefile".to_string());
        tool_names.push("LasToZlidar".to_string());
        tool_names.push("LidarClassifySubset".to_string());
        tool_names.push("LidarColourize".to_string());
        // tool_names.push("LidarConstructVectorTIN".to_string());
        tool_names.push("LidarDigitalSurfaceModel".to_string());
        tool_names.push("LidarElevationSlice".to_string());
        tool_names.push("LidarGroundPointFilter".to_string());
        tool_names.push("LidarHexBinning".to_string());
        tool_names.push("LidarHillshade".to_string());
        tool_names.push("LidarHistogram".to_string());
        tool_names.push("LidarIdwInterpolation".to_string());
        tool_names.push("LidarInfo".to_string());
        tool_names.push("LidarJoin".to_string());
        tool_names.push("LidarKappaIndex".to_string());
        tool_names.push("LidarNearestNeighbourGridding".to_string());
        tool_names.push("LidarPointDensity".to_string());
        tool_names.push("LidarPointStats".to_string());
        tool_names.push("LidarRbfInterpolation".to_string());
        tool_names.push("LidarRansacPlanes".to_string());
        tool_names.push("LidarRemoveDuplicates".to_string());
        tool_names.push("LidarRemoveOutliers".to_string());
        tool_names.push("LidarRooftopAnalysis".to_string());
        tool_names.push("LidarSegmentation".to_string());
        tool_names.push("LidarSegmentationBasedFilter".to_string());
        tool_names.push("LidarThin".to_string());
        tool_names.push("LidarThinHighDensity".to_string());
        tool_names.push("LidarTile".to_string());
        tool_names.push("LidarTileFootprint".to_string());
        tool_names.push("LidarTINGridding".to_string());
        tool_names.push("LidarTophatTransform".to_string());
        tool_names.push("NormalVectors".to_string());
        tool_names.push("SelectTilesByPolygon".to_string());
        tool_names.push("ZlidarToLas".to_string());

        // mathematical and statistical_analysis
        tool_names.push("AbsoluteValue".to_string());
        tool_names.push("Add".to_string());
        tool_names.push("And".to_string());
        tool_names.push("Anova".to_string());
        tool_names.push("ArcCos".to_string());
        tool_names.push("ArcSin".to_string());
        tool_names.push("ArcTan".to_string());
        tool_names.push("Atan2".to_string());
        tool_names.push("Arcosh".to_string());
        tool_names.push("Arsinh".to_string());
        tool_names.push("Artanh".to_string());
        tool_names.push("AttributeCorrelation".to_string());
        tool_names.push("AttributeCorrelationNeighbourhoodAnalysis".to_string());
        tool_names.push("AttributeHistogram".to_string());
        tool_names.push("AttributeScattergram".to_string());
        tool_names.push("Ceil".to_string());
        tool_names.push("Cos".to_string());
        tool_names.push("Cosh".to_string());
        tool_names.push("CrispnessIndex".to_string());
        tool_names.push("CrossTabulation".to_string());
        tool_names.push("CumulativeDistribution".to_string());
        tool_names.push("Decrement".to_string());
        tool_names.push("Divide".to_string());
        tool_names.push("EqualTo".to_string());
        tool_names.push("Exp".to_string());
        tool_names.push("Exp2".to_string());
        tool_names.push("ZonalStatistics".to_string());
        tool_names.push("Floor".to_string());
        tool_names.push("GreaterThan".to_string());
        tool_names.push("ImageAutocorrelation".to_string());
        tool_names.push("ImageCorrelation".to_string());
        tool_names.push("ImageCorrelationNeighbourhoodAnalysis".to_string());
        tool_names.push("ImageRegression".to_string());
        tool_names.push("Increment".to_string());
        tool_names.push("InPlaceAdd".to_string());
        tool_names.push("InPlaceDivide".to_string());
        tool_names.push("InPlaceMultiply".to_string());
        tool_names.push("InPlaceSubtract".to_string());
        tool_names.push("IntegerDivision".to_string());
        tool_names.push("IsNoData".to_string());
        tool_names.push("KappaIndex".to_string());
        tool_names.push("KsTestForNormality".to_string());
        tool_names.push("LessThan".to_string());
        tool_names.push("ListUniqueValues".to_string());
        tool_names.push("Log10".to_string());
        tool_names.push("Log2".to_string());
        tool_names.push("Ln".to_string());
        tool_names.push("Max".to_string());
        tool_names.push("Min".to_string());
        tool_names.push("Modulo".to_string());
        tool_names.push("Multiply".to_string());
        tool_names.push("Negate".to_string());
        tool_names.push("Not".to_string());
        tool_names.push("NotEqualTo".to_string());
        tool_names.push("Or".to_string());
        tool_names.push("PairedSampleTTest".to_string());
        tool_names.push("Power".to_string());
        tool_names.push("PrincipalComponentAnalysis".to_string());
        tool_names.push("Quantiles".to_string());
        tool_names.push("RandomField".to_string());
        tool_names.push("RandomSample".to_string());
        tool_names.push("RasterHistogram".to_string());
        tool_names.push("RasterSummaryStats".to_string());
        tool_names.push("Reciprocal".to_string());
        tool_names.push("RescaleValueRange".to_string());
        tool_names.push("RootMeanSquareError".to_string());
        tool_names.push("Round".to_string());
        tool_names.push("Sin".to_string());
        tool_names.push("Sinh".to_string());
        tool_names.push("Square".to_string());
        tool_names.push("SquareRoot".to_string());
        tool_names.push("Subtract".to_string());
        tool_names.push("Tan".to_string());
        tool_names.push("Tanh".to_string());
        tool_names.push("ToDegrees".to_string());
        tool_names.push("ToRadians".to_string());
        tool_names.push("TrendSurface".to_string());
        tool_names.push("TrendSurfaceVectorPoints".to_string());
        tool_names.push("Truncate".to_string());
        tool_names.push("TurningBandsSimulation".to_string());
        tool_names.push("TwoSampleKsTest".to_string());
        tool_names.push("WilcoxonSignedRankTest".to_string());
        tool_names.push("Xor".to_string());
        tool_names.push("ZScores".to_string());

        // stream_network_analysis
        tool_names.push("DistanceToOutlet".to_string());
        tool_names.push("ExtractStreams".to_string());
        tool_names.push("ExtractValleys".to_string());
        tool_names.push("FarthestChannelHead".to_string());
        tool_names.push("FindMainStem".to_string());
        tool_names.push("HackStreamOrder".to_string());
        tool_names.push("HortonStreamOrder".to_string());
        tool_names.push("LengthOfUpstreamChannels".to_string());
        tool_names.push("LongProfile".to_string());
        tool_names.push("LongProfileFromPoints".to_string());
        tool_names.push("RasterizeStreams".to_string());
        tool_names.push("RasterStreamsToVector".to_string());
        tool_names.push("RemoveShortStreams".to_string());
        tool_names.push("ShreveStreamMagnitude".to_string());
        tool_names.push("StrahlerStreamOrder".to_string());
        tool_names.push("StreamLinkClass".to_string());
        tool_names.push("StreamLinkIdentifier".to_string());
        tool_names.push("StreamLinkLength".to_string());
        tool_names.push("StreamLinkSlope".to_string());
        tool_names.push("StreamSlopeContinuous".to_string());
        tool_names.push("TopologicalStreamOrder".to_string());
        tool_names.push("TributaryIdentifier".to_string());

        // terrain_analysis
        tool_names.push("Aspect".to_string());
        tool_names.push("AverageNormalVectorAngularDeviation".to_string());
        tool_names.push("CircularVarianceOfAspect".to_string());
        tool_names.push("ContoursFromPoints".to_string());
        tool_names.push("ContoursFromRaster".to_string());
        tool_names.push("DevFromMeanElev".to_string());
        tool_names.push("DiffFromMeanElev".to_string());
        tool_names.push("DirectionalRelief".to_string());
        tool_names.push("DownslopeIndex".to_string());
        tool_names.push("EdgeDensity".to_string());
        tool_names.push("ElevAbovePit".to_string());
        tool_names.push("ElevPercentile".to_string());
        tool_names.push("ElevRelativeToMinMax".to_string());
        tool_names.push("ElevRelativeToWatershedMinMax".to_string());
        tool_names.push("EmbankmentMapping".to_string());
        tool_names.push("FeaturePreservingSmoothing".to_string());
        tool_names.push("FetchAnalysis".to_string());
        tool_names.push("FillMissingData".to_string());
        tool_names.push("FindRidges".to_string());
        tool_names.push("GaussianCurvature".to_string());
        tool_names.push("Geomorphons".to_string());
        tool_names.push("Hillshade".to_string());
        tool_names.push("HorizonAngle".to_string());
        tool_names.push("HypsometricAnalysis".to_string());
        tool_names.push("HypsometricallyTintedHillshade".to_string());
        tool_names.push("MapOffTerrainObjects".to_string());
        tool_names.push("MaxAnisotropyDev".to_string());
        tool_names.push("MaxAnisotropyDevSignature".to_string());
        tool_names.push("MaxBranchLength".to_string());
        tool_names.push("MaxDifferenceFromMean".to_string());
        tool_names.push("MaxDownslopeElevChange".to_string());
        tool_names.push("MaxElevDevSignature".to_string());
        tool_names.push("MaxElevationDeviation".to_string());
        tool_names.push("MaxUpslopeElevChange".to_string());
        tool_names.push("MaximalCurvature".to_string());
        tool_names.push("MeanCurvature".to_string());
        tool_names.push("MinDownslopeElevChange".to_string());
        tool_names.push("MinimalCurvature".to_string());
        tool_names.push("MultidirectionalHillshade".to_string());
        tool_names.push("MultiscaleElevationPercentile".to_string());
        tool_names.push("MultiscaleRoughness".to_string());
        tool_names.push("MultiscaleStdDevNormals".to_string());
        tool_names.push("MultiscaleStdDevNormalsSignature".to_string());
        tool_names.push("MultiscaleRoughnessSignature".to_string());
        tool_names.push("MultiscaleTopographicPositionImage".to_string());
        tool_names.push("NumDownslopeNeighbours".to_string());
        tool_names.push("NumUpslopeNeighbours".to_string());
        tool_names.push("PennockLandformClass".to_string());
        tool_names.push("PercentElevRange".to_string());
        tool_names.push("PlanCurvature".to_string());
        tool_names.push("ProfileCurvature".to_string());
        tool_names.push("Profile".to_string());
        tool_names.push("RelativeAspect".to_string());
        tool_names.push("StreamPowerIndex".to_string());
        tool_names.push("RelativeTopographicPosition".to_string());
        tool_names.push("RemoveOffTerrainObjects".to_string());
        tool_names.push("RuggednessIndex".to_string());
        tool_names.push("TimeInDaylight".to_string());
        tool_names.push("SedimentTransportIndex".to_string());
        tool_names.push("Slope".to_string());
        tool_names.push("SlopeVsElevationPlot".to_string());
        tool_names.push("SphericalStdDevOfNormals".to_string());
        tool_names.push("StandardDeviationOfSlope".to_string());
        tool_names.push("SurfaceAreaRatio".to_string());
        tool_names.push("TangentialCurvature".to_string());
        tool_names.push("TotalCurvature".to_string());
        tool_names.push("Viewshed".to_string());
        tool_names.push("VisibilityIndex".to_string());
        tool_names.push("WetnessIndex".to_string());

        tool_names.sort();

        let tm = ToolManager {
            working_dir: working_directory.to_string(),
            verbose: *verbose_mode,
            tool_names: tool_names,
        };
        Ok(tm)
    }

    fn get_tool(&self, tool_name: &str) -> Option<Box<dyn WhiteboxTool + 'static>> {
        match tool_name.to_lowercase().replace("_", "").as_ref() {
            // data_tools
            "addpointcoordinatestotable" => {
                Some(Box::new(data_tools::AddPointCoordinatesToTable::new()))
            }
            "cleanvector" => Some(Box::new(data_tools::CleanVector::new())),
            "convertnodatatozero" => Some(Box::new(data_tools::ConvertNodataToZero::new())),
            "convertrasterformat" => Some(Box::new(data_tools::ConvertRasterFormat::new())),
            "csvpointstovector" => Some(Box::new(data_tools::CsvPointsToVector::new())),
            "exporttabletocsv" => Some(Box::new(data_tools::ExportTableToCsv::new())),
            "jointables" => Some(Box::new(data_tools::JoinTables::new())),
            "linestopolygons" => Some(Box::new(data_tools::LinesToPolygons::new())),
            "mergetablewithcsv" => Some(Box::new(data_tools::MergeTableWithCsv::new())),
            "mergevectors" => Some(Box::new(data_tools::MergeVectors::new())),
            "modifynodatavalue" => Some(Box::new(data_tools::ModifyNoDataValue::new())),
            "multiparttosinglepart" => Some(Box::new(data_tools::MultiPartToSinglePart::new())),
            "newrasterfrombase" => Some(Box::new(data_tools::NewRasterFromBase::new())),
            "polygonstolines" => Some(Box::new(data_tools::PolygonsToLines::new())),
            "printgeotifftags" => Some(Box::new(data_tools::PrintGeoTiffTags::new())),
            "rastertovectorlines" => Some(Box::new(data_tools::RasterToVectorLines::new())),
            "rastertovectorpoints" => Some(Box::new(data_tools::RasterToVectorPoints::new())),
            "rastertovectorpolygons" => Some(Box::new(data_tools::RasterToVectorPolygons::new())),
            "reinitializeattributetable" => {
                Some(Box::new(data_tools::ReinitializeAttributeTable::new()))
            }
            "removepolygonholes" => Some(Box::new(data_tools::RemovePolygonHoles::new())),
            "setnodatavalue" => Some(Box::new(data_tools::SetNodataValue::new())),
            "singleparttomultipart" => Some(Box::new(data_tools::SinglePartToMultiPart::new())),
            "vectorlinestoraster" => Some(Box::new(data_tools::VectorLinesToRaster::new())),
            "vectorpointstoraster" => Some(Box::new(data_tools::VectorPointsToRaster::new())),
            "vectorpolygonstoraster" => Some(Box::new(data_tools::VectorPolygonsToRaster::new())),

            // gis_analysis
            "aggregateraster" => Some(Box::new(gis_analysis::AggregateRaster::new())),
            "averageoverlay" => Some(Box::new(gis_analysis::AverageOverlay::new())),
            "blockmaximumgridding" => Some(Box::new(gis_analysis::BlockMaximumGridding::new())),
            "blockminimumgridding" => Some(Box::new(gis_analysis::BlockMinimumGridding::new())),
            "boundaryshapecomplexity" => {
                Some(Box::new(gis_analysis::BoundaryShapeComplexity::new()))
            }
            "bufferraster" => Some(Box::new(gis_analysis::BufferRaster::new())),
            // "buffervector" => Some(Box::new(gis_analysis::BufferVector::new())),
            "centroid" => Some(Box::new(gis_analysis::Centroid::new())),
            "centroidvector" => Some(Box::new(gis_analysis::CentroidVector::new())),
            "clip" => Some(Box::new(gis_analysis::Clip::new())),
            "cliprastertopolygon" => Some(Box::new(gis_analysis::ClipRasterToPolygon::new())),
            "clump" => Some(Box::new(gis_analysis::Clump::new())),
            "compactnessratio" => Some(Box::new(gis_analysis::CompactnessRatio::new())),
            "constructvectortin" => Some(Box::new(gis_analysis::ConstructVectorTIN::new())),
            "countif" => Some(Box::new(gis_analysis::CountIf::new())),
            "costallocation" => Some(Box::new(gis_analysis::CostAllocation::new())),
            "costdistance" => Some(Box::new(gis_analysis::CostDistance::new())),
            "costpathway" => Some(Box::new(gis_analysis::CostPathway::new())),
            "createhexagonalvectorgrid" => {
                Some(Box::new(gis_analysis::CreateHexagonalVectorGrid::new()))
            }
            "createplane" => Some(Box::new(gis_analysis::CreatePlane::new())),
            "createrectangularvectorgrid" => {
                Some(Box::new(gis_analysis::CreateRectangularVectorGrid::new()))
            }
            "difference" => Some(Box::new(gis_analysis::Difference::new())),
            "dissolve" => Some(Box::new(gis_analysis::Dissolve::new())),
            "edgeproportion" => Some(Box::new(gis_analysis::EdgeProportion::new())),
            "eliminatecoincidentpoints" => {
                Some(Box::new(gis_analysis::EliminateCoincidentPoints::new()))
            }
            "elongationratio" => Some(Box::new(gis_analysis::ElongationRatio::new())),
            "erase" => Some(Box::new(gis_analysis::Erase::new())),
            "erasepolygonfromraster" => Some(Box::new(gis_analysis::ErasePolygonFromRaster::new())),
            "euclideanallocation" => Some(Box::new(gis_analysis::EuclideanAllocation::new())),
            "euclideandistance" => Some(Box::new(gis_analysis::EuclideanDistance::new())),
            "extendvectorlines" => Some(Box::new(gis_analysis::ExtendVectorLines::new())),
            "extractnodes" => Some(Box::new(gis_analysis::ExtractNodes::new())),
            "extractrastervaluesatpoints" => {
                Some(Box::new(gis_analysis::ExtractRasterValuesAtPoints::new()))
            }
            "filterrasterfeaturesbyarea" => {
                Some(Box::new(gis_analysis::FilterRasterFeaturesByArea::new()))
            }
            "findlowestorhighestpoints" => {
                Some(Box::new(gis_analysis::FindLowestOrHighestPoints::new()))
            }
            "findpatchorclassedgecells" => {
                Some(Box::new(gis_analysis::FindPatchOrClassEdgeCells::new()))
            }
            "highestposition" => Some(Box::new(gis_analysis::HighestPosition::new())),
            "holeproportion" => Some(Box::new(gis_analysis::HoleProportion::new())),
            "idwinterpolation" => Some(Box::new(gis_analysis::IdwInterpolation::new())),
            "intersect" => Some(Box::new(gis_analysis::Intersect::new())),
            "layerfootprint" => Some(Box::new(gis_analysis::LayerFootprint::new())),
            "lineintersections" => Some(Box::new(gis_analysis::LineIntersections::new())),
            "linearityindex" => Some(Box::new(gis_analysis::LinearityIndex::new())),
            "lowestposition" => Some(Box::new(gis_analysis::LowestPosition::new())),
            "maxabsoluteoverlay" => Some(Box::new(gis_analysis::MaxAbsoluteOverlay::new())),
            "maxoverlay" => Some(Box::new(gis_analysis::MaxOverlay::new())),
            "medoid" => Some(Box::new(gis_analysis::Medoid::new())),
            "mergelinesegments" => Some(Box::new(gis_analysis::MergeLineSegments::new())),
            "minabsoluteoverlay" => Some(Box::new(gis_analysis::MinAbsoluteOverlay::new())),
            "minimumboundingbox" => Some(Box::new(gis_analysis::MinimumBoundingBox::new())),
            "minimumboundingcircle" => Some(Box::new(gis_analysis::MinimumBoundingCircle::new())),
            "minimumboundingenvelope" => {
                Some(Box::new(gis_analysis::MinimumBoundingEnvelope::new()))
            }
            "minimumconvexhull" => Some(Box::new(gis_analysis::MinimumConvexHull::new())),
            "minoverlay" => Some(Box::new(gis_analysis::MinOverlay::new())),
            "multiplyoverlay" => Some(Box::new(gis_analysis::MultiplyOverlay::new())),
            "naturalneighbourinterpolation" => {
                Some(Box::new(gis_analysis::NaturalNeighbourInterpolation::new()))
            }
            "nearestneighbourgridding" => {
                Some(Box::new(gis_analysis::NearestNeighbourGridding::new()))
            }
            "narrownessindex" => Some(Box::new(gis_analysis::NarrownessIndex::new())),
            "patchorientation" => Some(Box::new(gis_analysis::PatchOrientation::new())),
            "percentequalto" => Some(Box::new(gis_analysis::PercentEqualTo::new())),
            "percentgreaterthan" => Some(Box::new(gis_analysis::PercentGreaterThan::new())),
            "percentlessthan" => Some(Box::new(gis_analysis::PercentLessThan::new())),
            "perimeterarearatio" => Some(Box::new(gis_analysis::PerimeterAreaRatio::new())),
            "pickfromlist" => Some(Box::new(gis_analysis::PickFromList::new())),
            "polygonarea" => Some(Box::new(gis_analysis::PolygonArea::new())),
            "polygonlongaxis" => Some(Box::new(gis_analysis::PolygonLongAxis::new())),
            "polygonperimeter" => Some(Box::new(gis_analysis::PolygonPerimeter::new())),
            "polygonshortaxis" => Some(Box::new(gis_analysis::PolygonShortAxis::new())),
            "polygonize" => Some(Box::new(gis_analysis::Polygonize::new())),
            "radialbasisfunctioninterpolation" => Some(Box::new(
                gis_analysis::RadialBasisFunctionInterpolation::new(),
            )),
            "radiusofgyration" => Some(Box::new(gis_analysis::RadiusOfGyration::new())),
            "rasterarea" => Some(Box::new(gis_analysis::RasterArea::new())),
            "rastercellassignment" => Some(Box::new(gis_analysis::RasterCellAssignment::new())),
            "rasterperimeter" => Some(Box::new(gis_analysis::RasterPerimeter::new())),
            "reclass" => Some(Box::new(gis_analysis::Reclass::new())),
            "reclassequalinterval" => Some(Box::new(gis_analysis::ReclassEqualInterval::new())),
            "reclassfromfile" => Some(Box::new(gis_analysis::ReclassFromFile::new())),
            "relatedcircumscribingcircle" => {
                Some(Box::new(gis_analysis::RelatedCircumscribingCircle::new()))
            }
            "shapecomplexityindex" => Some(Box::new(gis_analysis::ShapeComplexityIndex::new())),
            "shapecomplexityindexraster" => {
                Some(Box::new(gis_analysis::ShapeComplexityIndexRaster::new()))
            }
            "smoothvectors" => Some(Box::new(gis_analysis::SmoothVectors::new())),
            "splitwithlines" => Some(Box::new(gis_analysis::SplitWithLines::new())),
            "sumoverlay" => Some(Box::new(gis_analysis::SumOverlay::new())),
            "symmetricaldifference" => Some(Box::new(gis_analysis::SymmetricalDifference::new())),
            "tingridding" => Some(Box::new(gis_analysis::TINGridding::new())),
            "union" => Some(Box::new(gis_analysis::Union::new())),
            "updatenodatacells" => Some(Box::new(gis_analysis::UpdateNodataCells::new())),
            "vectorhexbinning" => Some(Box::new(gis_analysis::VectorHexBinning::new())),
            "voronoidiagram" => Some(Box::new(gis_analysis::VoronoiDiagram::new())),
            "weightedoverlay" => Some(Box::new(gis_analysis::WeightedOverlay::new())),
            "weightedsum" => Some(Box::new(gis_analysis::WeightedSum::new())),

            // hydro_analysis
            "averageflowpathslope" => Some(Box::new(hydro_analysis::AverageFlowpathSlope::new())),
            "averageupslopeflowpathlength" => {
                Some(Box::new(hydro_analysis::AverageUpslopeFlowpathLength::new()))
            }
            "basins" => Some(Box::new(hydro_analysis::Basins::new())),
            "breachdepressions" => Some(Box::new(hydro_analysis::BreachDepressions::new())),
            "breachdepressionsleastcost" => {
                Some(Box::new(hydro_analysis::BreachDepressionsLeastCost::new()))
            }
            "breachsinglecellpits" => Some(Box::new(hydro_analysis::BreachSingleCellPits::new())),
            "burnstreamsatroads" => Some(Box::new(hydro_analysis::BurnStreamsAtRoads::new())),
            "d8flowaccumulation" => Some(Box::new(hydro_analysis::D8FlowAccumulation::new())),
            "d8massflux" => Some(Box::new(hydro_analysis::D8MassFlux::new())),
            "d8pointer" => Some(Box::new(hydro_analysis::D8Pointer::new())),
            "depthinsink" => Some(Box::new(hydro_analysis::DepthInSink::new())),
            "dinfflowaccumulation" => Some(Box::new(hydro_analysis::DInfFlowAccumulation::new())),
            "dinfmassflux" => Some(Box::new(hydro_analysis::DInfMassFlux::new())),
            "dinfpointer" => Some(Box::new(hydro_analysis::DInfPointer::new())),
            "downslopedistancetostream" => {
                Some(Box::new(hydro_analysis::DownslopeDistanceToStream::new()))
            }
            "downslopeflowpathlength" => {
                Some(Box::new(hydro_analysis::DownslopeFlowpathLength::new()))
            }
            "elevationabovestream" => Some(Box::new(hydro_analysis::ElevationAboveStream::new())),
            "elevationabovestreameuclidean" => Some(Box::new(
                hydro_analysis::ElevationAboveStreamEuclidean::new(),
            )),
            "fd8flowaccumulation" => Some(Box::new(hydro_analysis::FD8FlowAccumulation::new())),
            "fd8pointer" => Some(Box::new(hydro_analysis::FD8Pointer::new())),
            "fillburn" => Some(Box::new(hydro_analysis::FillBurn::new())),
            "filldepressions" => Some(Box::new(hydro_analysis::FillDepressions::new())),
            "filldepressionsplanchonanddarboux" => Some(Box::new(
                hydro_analysis::FillDepressionsPlanchonAndDarboux::new(),
            )),
            "filldepressionswangandliu" => {
                Some(Box::new(hydro_analysis::FillDepressionsWangAndLiu::new()))
            }
            "fillsinglecellpits" => Some(Box::new(hydro_analysis::FillSingleCellPits::new())),
            "findnoflowcells" => Some(Box::new(hydro_analysis::FindNoFlowCells::new())),
            "findparallelflow" => Some(Box::new(hydro_analysis::FindParallelFlow::new())),
            "flattenlakes" => Some(Box::new(hydro_analysis::FlattenLakes::new())),
            "floodorder" => Some(Box::new(hydro_analysis::FloodOrder::new())),
            "flowaccumulationfullworkflow" => {
                Some(Box::new(hydro_analysis::FlowAccumulationFullWorkflow::new()))
            }
            "flowlengthdiff" => Some(Box::new(hydro_analysis::FlowLengthDiff::new())),
            "hillslopes" => Some(Box::new(hydro_analysis::Hillslopes::new())),
            "impoundmentsizeindex" => Some(Box::new(hydro_analysis::ImpoundmentSizeIndex::new())),
            "insertdams" => Some(Box::new(hydro_analysis::InsertDams::new())),
            "isobasins" => Some(Box::new(hydro_analysis::Isobasins::new())),
            "jensonsnappourpoints" => Some(Box::new(hydro_analysis::JensonSnapPourPoints::new())),
            "longestflowpath" => Some(Box::new(hydro_analysis::LongestFlowpath::new())),
            "maxupslopeflowpathlength" => {
                Some(Box::new(hydro_analysis::MaxUpslopeFlowpathLength::new()))
            }
            "mdinfflowaccumulation" => Some(Box::new(hydro_analysis::MDInfFlowAccumulation::new())),
            "numinflowingneighbours" => {
                Some(Box::new(hydro_analysis::NumInflowingNeighbours::new()))
            }
            "raisewalls" => Some(Box::new(hydro_analysis::RaiseWalls::new())),
            "rho8pointer" => Some(Box::new(hydro_analysis::Rho8Pointer::new())),
            "sink" => Some(Box::new(hydro_analysis::Sink::new())),
            "snappourpoints" => Some(Box::new(hydro_analysis::SnapPourPoints::new())),
            "stochasticdepressionanalysis" => {
                Some(Box::new(hydro_analysis::StochasticDepressionAnalysis::new()))
            }
            "strahlerorderbasins" => Some(Box::new(hydro_analysis::StrahlerOrderBasins::new())),
            "subbasins" => Some(Box::new(hydro_analysis::Subbasins::new())),
            "tracedownslopeflowpaths" => {
                Some(Box::new(hydro_analysis::TraceDownslopeFlowpaths::new()))
            }
            "unnestbasins" => Some(Box::new(hydro_analysis::UnnestBasins::new())),
            "upslopedepressionstorage" => {
                Some(Box::new(hydro_analysis::UpslopeDepressionStorage::new()))
            }
            "watershed" => Some(Box::new(hydro_analysis::Watershed::new())),

            // image_analysis
            "adaptivefilter" => Some(Box::new(image_analysis::AdaptiveFilter::new())),
            "balancecontrastenhancement" => {
                Some(Box::new(image_analysis::BalanceContrastEnhancement::new()))
            }
            "bilateralfilter" => Some(Box::new(image_analysis::BilateralFilter::new())),
            "changevectoranalysis" => Some(Box::new(image_analysis::ChangeVectorAnalysis::new())),
            "closing" => Some(Box::new(image_analysis::Closing::new())),
            "cornerdetection" => Some(Box::new(image_analysis::CornerDetection::new())),
            "correctvignetting" => Some(Box::new(image_analysis::CorrectVignetting::new())),
            "conservativesmoothingfilter" => {
                Some(Box::new(image_analysis::ConservativeSmoothingFilter::new()))
            }
            "createcolourcomposite" => Some(Box::new(image_analysis::CreateColourComposite::new())),
            "directdecorrelationstretch" => {
                Some(Box::new(image_analysis::DirectDecorrelationStretch::new()))
            }
            "diversityfilter" => Some(Box::new(image_analysis::DiversityFilter::new())),
            "diffofgaussianfilter" => Some(Box::new(image_analysis::DiffOfGaussianFilter::new())),
            "edgepreservingmeanfilter" => {
                Some(Box::new(image_analysis::EdgePreservingMeanFilter::new()))
            }
            "embossfilter" => Some(Box::new(image_analysis::EmbossFilter::new())),
            "fastalmostgaussianfilter" => {
                Some(Box::new(image_analysis::FastAlmostGaussianFilter::new()))
            }
            "flipimage" => Some(Box::new(image_analysis::FlipImage::new())),
            "gammacorrection" => Some(Box::new(image_analysis::GammaCorrection::new())),
            "gaussiancontraststretch" => {
                Some(Box::new(image_analysis::GaussianContrastStretch::new()))
            }
            "gaussianfilter" => Some(Box::new(image_analysis::GaussianFilter::new())),
            "highpassfilter" => Some(Box::new(image_analysis::HighPassFilter::new())),
            "highpassmedianfilter" => Some(Box::new(image_analysis::HighPassMedianFilter::new())),
            "histogramequalization" => Some(Box::new(image_analysis::HistogramEqualization::new())),
            "histogrammatching" => Some(Box::new(image_analysis::HistogramMatching::new())),
            "histogrammatchingtwoimages" => {
                Some(Box::new(image_analysis::HistogramMatchingTwoImages::new()))
            }
            "ihstorgb" => Some(Box::new(image_analysis::IhsToRgb::new())),
            "imagestackprofile" => Some(Box::new(image_analysis::ImageStackProfile::new())),
            "integralimage" => Some(Box::new(image_analysis::IntegralImage::new())),
            "kmeansclustering" => Some(Box::new(image_analysis::KMeansClustering::new())),
            "knearestmeanfilter" => Some(Box::new(image_analysis::KNearestMeanFilter::new())),
            "laplacianfilter" => Some(Box::new(image_analysis::LaplacianFilter::new())),
            "laplacianofgaussianfilter" => {
                Some(Box::new(image_analysis::LaplacianOfGaussianFilter::new()))
            }
            "leesigmafilter" => Some(Box::new(image_analysis::LeeSigmaFilter::new())),
            "linedetectionfilter" => Some(Box::new(image_analysis::LineDetectionFilter::new())),
            "linethinning" => Some(Box::new(image_analysis::LineThinning::new())),
            "majorityfilter" => Some(Box::new(image_analysis::MajorityFilter::new())),
            "maximumfilter" => Some(Box::new(image_analysis::MaximumFilter::new())),
            "minmaxcontraststretch" => Some(Box::new(image_analysis::MinMaxContrastStretch::new())),
            "meanfilter" => Some(Box::new(image_analysis::MeanFilter::new())),
            "medianfilter" => Some(Box::new(image_analysis::MedianFilter::new())),
            "minimumfilter" => Some(Box::new(image_analysis::MinimumFilter::new())),
            "modifiedkmeansclustering" => {
                Some(Box::new(image_analysis::ModifiedKMeansClustering::new()))
            }
            "mosaic" => Some(Box::new(image_analysis::Mosaic::new())),
            "mosaicwithfeathering" => Some(Box::new(image_analysis::MosaicWithFeathering::new())),
            "normalizeddifferenceindex" => {
                Some(Box::new(image_analysis::NormalizedDifferenceIndex::new()))
            }
            "olympicfilter" => Some(Box::new(image_analysis::OlympicFilter::new())),
            "opening" => Some(Box::new(image_analysis::Opening::new())),
            "panchromaticsharpening" => {
                Some(Box::new(image_analysis::PanchromaticSharpening::new()))
            }
            "percentagecontraststretch" => {
                Some(Box::new(image_analysis::PercentageContrastStretch::new()))
            }
            "percentilefilter" => Some(Box::new(image_analysis::PercentileFilter::new())),
            "prewittfilter" => Some(Box::new(image_analysis::PrewittFilter::new())),
            "rangefilter" => Some(Box::new(image_analysis::RangeFilter::new())),
            "removespurs" => Some(Box::new(image_analysis::RemoveSpurs::new())),
            "resample" => Some(Box::new(image_analysis::Resample::new())),
            "rgbtoihs" => Some(Box::new(image_analysis::RgbToIhs::new())),
            "robertscrossfilter" => Some(Box::new(image_analysis::RobertsCrossFilter::new())),
            "scharrfilter" => Some(Box::new(image_analysis::ScharrFilter::new())),
            "sigmoidalcontraststretch" => {
                Some(Box::new(image_analysis::SigmoidalContrastStretch::new()))
            }
            "sobelfilter" => Some(Box::new(image_analysis::SobelFilter::new())),
            "splitcolourcomposite" => Some(Box::new(image_analysis::SplitColourComposite::new())),
            "standarddeviationcontraststretch" => Some(Box::new(
                image_analysis::StandardDeviationContrastStretch::new(),
            )),
            "standarddeviationfilter" => {
                Some(Box::new(image_analysis::StandardDeviationFilter::new()))
            }
            "thickenrasterline" => Some(Box::new(image_analysis::ThickenRasterLine::new())),
            "tophattransform" => Some(Box::new(image_analysis::TophatTransform::new())),
            "totalfilter" => Some(Box::new(image_analysis::TotalFilter::new())),
            "unsharpmasking" => Some(Box::new(image_analysis::UnsharpMasking::new())),
            "userdefinedweightsfilter" => {
                Some(Box::new(image_analysis::UserDefinedWeightsFilter::new()))
            }
            "writefunctionmemoryinsertion" => {
                Some(Box::new(image_analysis::WriteFunctionMemoryInsertion::new()))
            }

            // lidar_analysis
            "asciitolas" => Some(Box::new(lidar_analysis::AsciiToLas::new())),
            "lidarblockmaximum" => Some(Box::new(lidar_analysis::LidarBlockMaximum::new())),
            "lidarblockminimum" => Some(Box::new(lidar_analysis::LidarBlockMinimum::new())),
            "classifybuildingsinlidar" => {
                Some(Box::new(lidar_analysis::ClassifyBuildingsInLidar::new()))
            }
            "classifyoverlappoints" => Some(Box::new(lidar_analysis::ClassifyOverlapPoints::new())),
            "cliplidartopolygon" => Some(Box::new(lidar_analysis::ClipLidarToPolygon::new())),
            // "contourlidar" => Some(Box::new(lidar_analysis::ContourLidar::new())),
            "erasepolygonfromlidar" => Some(Box::new(lidar_analysis::ErasePolygonFromLidar::new())),
            "filterlidarclasses" => Some(Box::new(lidar_analysis::FilterLidarClasses::new())),
            "filterlidarscanangles" => Some(Box::new(lidar_analysis::FilterLidarScanAngles::new())),
            "findflightlineedgepoints" => {
                Some(Box::new(lidar_analysis::FindFlightlineEdgePoints::new()))
            }
            "flightlineoverlap" => Some(Box::new(lidar_analysis::FlightlineOverlap::new())),
            "heightaboveground" => Some(Box::new(lidar_analysis::HeightAboveGround::new())),
            "lastoascii" => Some(Box::new(lidar_analysis::LasToAscii::new())),
            "lastomultipointshapefile" => {
                Some(Box::new(lidar_analysis::LasToMultipointShapefile::new()))
            }
            "lastoshapefile" => Some(Box::new(lidar_analysis::LasToShapefile::new())),
            "lastozlidar" => Some(Box::new(lidar_analysis::LasToZlidar::new())),
            "lidarclassifysubset" => Some(Box::new(lidar_analysis::LidarClassifySubset::new())),
            "lidarcolourize" => Some(Box::new(lidar_analysis::LidarColourize::new())),
            // "lidarconstructvectortin" => {
            //     Some(Box::new(lidar_analysis::LidarConstructVectorTIN::new()))
            // }
            "lidardigitalsurfacemodel" => {
                Some(Box::new(lidar_analysis::LidarDigitalSurfaceModel::new()))
            }
            "lidarelevationslice" => Some(Box::new(lidar_analysis::LidarElevationSlice::new())),
            "lidargroundpointfilter" => {
                Some(Box::new(lidar_analysis::LidarGroundPointFilter::new()))
            }
            "lidarhexbinning" => Some(Box::new(lidar_analysis::LidarHexBinning::new())),
            "lidarhillshade" => Some(Box::new(lidar_analysis::LidarHillshade::new())),
            "lidarhistogram" => Some(Box::new(lidar_analysis::LidarHistogram::new())),
            "lidaridwinterpolation" => Some(Box::new(lidar_analysis::LidarIdwInterpolation::new())),
            "lidarinfo" => Some(Box::new(lidar_analysis::LidarInfo::new())),
            "lidarjoin" => Some(Box::new(lidar_analysis::LidarJoin::new())),
            "lidarkappaindex" => Some(Box::new(lidar_analysis::LidarKappaIndex::new())),
            "lidarnearestneighbourgridding" => Some(Box::new(
                lidar_analysis::LidarNearestNeighbourGridding::new(),
            )),
            "lidarpointdensity" => Some(Box::new(lidar_analysis::LidarPointDensity::new())),
            "lidarpointstats" => Some(Box::new(lidar_analysis::LidarPointStats::new())),
            "lidarrbfinterpolation" => Some(Box::new(lidar_analysis::LidarRbfInterpolation::new())),
            "lidarransacplanes" => Some(Box::new(lidar_analysis::LidarRansacPlanes::new())),
            "lidarremoveduplicates" => Some(Box::new(lidar_analysis::LidarRemoveDuplicates::new())),
            "lidarremoveoutliers" => Some(Box::new(lidar_analysis::LidarRemoveOutliers::new())),
            "lidarrooftopanalysis" => Some(Box::new(lidar_analysis::LidarRooftopAnalysis::new())),
            "lidarsegmentation" => Some(Box::new(lidar_analysis::LidarSegmentation::new())),
            "lidarsegmentationbasedfilter" => {
                Some(Box::new(lidar_analysis::LidarSegmentationBasedFilter::new()))
            }
            "lidarthin" => Some(Box::new(lidar_analysis::LidarThin::new())),
            "lidarthinhighdensity" => Some(Box::new(lidar_analysis::LidarThinHighDensity::new())),
            "lidartile" => Some(Box::new(lidar_analysis::LidarTile::new())),
            "lidartilefootprint" => Some(Box::new(lidar_analysis::LidarTileFootprint::new())),
            "lidartingridding" => Some(Box::new(lidar_analysis::LidarTINGridding::new())),
            "lidartophattransform" => Some(Box::new(lidar_analysis::LidarTophatTransform::new())),
            "normalvectors" => Some(Box::new(lidar_analysis::NormalVectors::new())),
            "selecttilesbypolygon" => Some(Box::new(lidar_analysis::SelectTilesByPolygon::new())),
            "zlidartolas" => Some(Box::new(lidar_analysis::ZlidarToLas::new())),

            // mathematical and statistical_analysis
            "absolutevalue" => Some(Box::new(math_stat_analysis::AbsoluteValue::new())),
            "add" => Some(Box::new(math_stat_analysis::Add::new())),
            "and" => Some(Box::new(math_stat_analysis::And::new())),
            "anova" => Some(Box::new(math_stat_analysis::Anova::new())),
            "arccos" => Some(Box::new(math_stat_analysis::ArcCos::new())),
            "arcsin" => Some(Box::new(math_stat_analysis::ArcSin::new())),
            "arctan" => Some(Box::new(math_stat_analysis::ArcTan::new())),
            "atan2" => Some(Box::new(math_stat_analysis::Atan2::new())),
            "arcosh" => Some(Box::new(math_stat_analysis::Arcosh::new())),
            "arsinh" => Some(Box::new(math_stat_analysis::Arsinh::new())),
            "artanh" => Some(Box::new(math_stat_analysis::Artanh::new())),
            "attributecorrelation" => {
                Some(Box::new(math_stat_analysis::AttributeCorrelation::new()))
            }
            "attributecorrelationneighbourhoodanalysis" => Some(Box::new(
                math_stat_analysis::AttributeCorrelationNeighbourhoodAnalysis::new(),
            )),
            "attributehistogram" => Some(Box::new(math_stat_analysis::AttributeHistogram::new())),
            "attributescattergram" => {
                Some(Box::new(math_stat_analysis::AttributeScattergram::new()))
            }
            "ceil" => Some(Box::new(math_stat_analysis::Ceil::new())),
            "cos" => Some(Box::new(math_stat_analysis::Cos::new())),
            "cosh" => Some(Box::new(math_stat_analysis::Cosh::new())),
            "crispnessindex" => Some(Box::new(math_stat_analysis::CrispnessIndex::new())),
            "crosstabulation" => Some(Box::new(math_stat_analysis::CrossTabulation::new())),
            "cumulativedistribution" => {
                Some(Box::new(math_stat_analysis::CumulativeDistribution::new()))
            }
            "decrement" => Some(Box::new(math_stat_analysis::Decrement::new())),
            "divide" => Some(Box::new(math_stat_analysis::Divide::new())),
            "equalto" => Some(Box::new(math_stat_analysis::EqualTo::new())),
            "exp" => Some(Box::new(math_stat_analysis::Exp::new())),
            "exp2" => Some(Box::new(math_stat_analysis::Exp2::new())),
            "zonalstatistics" => Some(Box::new(math_stat_analysis::ZonalStatistics::new())),
            "floor" => Some(Box::new(math_stat_analysis::Floor::new())),
            "greaterthan" => Some(Box::new(math_stat_analysis::GreaterThan::new())),
            "imageautocorrelation" => {
                Some(Box::new(math_stat_analysis::ImageAutocorrelation::new()))
            }
            "imagecorrelation" => Some(Box::new(math_stat_analysis::ImageCorrelation::new())),
            "imagecorrelationneighbourhoodanalysis" => Some(Box::new(
                math_stat_analysis::ImageCorrelationNeighbourhoodAnalysis::new(),
            )),
            "imageregression" => Some(Box::new(math_stat_analysis::ImageRegression::new())),
            "increment" => Some(Box::new(math_stat_analysis::Increment::new())),
            "inplaceadd" => Some(Box::new(math_stat_analysis::InPlaceAdd::new())),
            "inplacedivide" => Some(Box::new(math_stat_analysis::InPlaceDivide::new())),
            "inplacemultiply" => Some(Box::new(math_stat_analysis::InPlaceMultiply::new())),
            "inplacesubtract" => Some(Box::new(math_stat_analysis::InPlaceSubtract::new())),
            "integerdivision" => Some(Box::new(math_stat_analysis::IntegerDivision::new())),
            "isnodata" => Some(Box::new(math_stat_analysis::IsNoData::new())),
            "kappaindex" => Some(Box::new(math_stat_analysis::KappaIndex::new())),
            "kstestfornormality" => Some(Box::new(math_stat_analysis::KsTestForNormality::new())),
            "lessthan" => Some(Box::new(math_stat_analysis::LessThan::new())),
            "listuniquevalues" => Some(Box::new(math_stat_analysis::ListUniqueValues::new())),
            "log10" => Some(Box::new(math_stat_analysis::Log10::new())),
            "log2" => Some(Box::new(math_stat_analysis::Log2::new())),
            "ln" => Some(Box::new(math_stat_analysis::Ln::new())),
            "max" => Some(Box::new(math_stat_analysis::Max::new())),
            "min" => Some(Box::new(math_stat_analysis::Min::new())),
            "modulo" => Some(Box::new(math_stat_analysis::Modulo::new())),
            "multiply" => Some(Box::new(math_stat_analysis::Multiply::new())),
            "negate" => Some(Box::new(math_stat_analysis::Negate::new())),
            "not" => Some(Box::new(math_stat_analysis::Not::new())),
            "notequalto" => Some(Box::new(math_stat_analysis::NotEqualTo::new())),
            "or" => Some(Box::new(math_stat_analysis::Or::new())),
            "pairedsamplettest" => Some(Box::new(math_stat_analysis::PairedSampleTTest::new())),
            "power" => Some(Box::new(math_stat_analysis::Power::new())),
            "principalcomponentanalysis" => Some(Box::new(
                math_stat_analysis::PrincipalComponentAnalysis::new(),
            )),
            "quantiles" => Some(Box::new(math_stat_analysis::Quantiles::new())),
            "randomfield" => Some(Box::new(math_stat_analysis::RandomField::new())),
            "randomsample" => Some(Box::new(math_stat_analysis::RandomSample::new())),
            "rasterhistogram" => Some(Box::new(math_stat_analysis::RasterHistogram::new())),
            "rastersummarystats" => Some(Box::new(math_stat_analysis::RasterSummaryStats::new())),
            "reciprocal" => Some(Box::new(math_stat_analysis::Reciprocal::new())),
            "rescalevaluerange" => Some(Box::new(math_stat_analysis::RescaleValueRange::new())),
            "rootmeansquareerror" => Some(Box::new(math_stat_analysis::RootMeanSquareError::new())),
            "round" => Some(Box::new(math_stat_analysis::Round::new())),
            "sin" => Some(Box::new(math_stat_analysis::Sin::new())),
            "sinh" => Some(Box::new(math_stat_analysis::Sinh::new())),
            "square" => Some(Box::new(math_stat_analysis::Square::new())),
            "squareroot" => Some(Box::new(math_stat_analysis::SquareRoot::new())),
            "subtract" => Some(Box::new(math_stat_analysis::Subtract::new())),
            "tan" => Some(Box::new(math_stat_analysis::Tan::new())),
            "tanh" => Some(Box::new(math_stat_analysis::Tanh::new())),
            "todegrees" => Some(Box::new(math_stat_analysis::ToDegrees::new())),
            "toradians" => Some(Box::new(math_stat_analysis::ToRadians::new())),
            "trendsurface" => Some(Box::new(math_stat_analysis::TrendSurface::new())),
            "trendsurfacevectorpoints" => {
                Some(Box::new(math_stat_analysis::TrendSurfaceVectorPoints::new()))
            }
            "truncate" => Some(Box::new(math_stat_analysis::Truncate::new())),
            "turningbandssimulation" => {
                Some(Box::new(math_stat_analysis::TurningBandsSimulation::new()))
            }
            "twosamplekstest" => Some(Box::new(math_stat_analysis::TwoSampleKsTest::new())),
            "wilcoxonsignedranktest" => {
                Some(Box::new(math_stat_analysis::WilcoxonSignedRankTest::new()))
            }
            "xor" => Some(Box::new(math_stat_analysis::Xor::new())),
            "zscores" => Some(Box::new(math_stat_analysis::ZScores::new())),

            // stream_network_analysis
            "distancetooutlet" => Some(Box::new(stream_network_analysis::DistanceToOutlet::new())),
            "extractstreams" => Some(Box::new(stream_network_analysis::ExtractStreams::new())),
            "extractvalleys" => Some(Box::new(stream_network_analysis::ExtractValleys::new())),
            "farthestchannelhead" => {
                Some(Box::new(stream_network_analysis::FarthestChannelHead::new()))
            }
            "findmainstem" => Some(Box::new(stream_network_analysis::FindMainStem::new())),
            "hackstreamorder" => Some(Box::new(stream_network_analysis::HackStreamOrder::new())),
            "hortonstreamorder" => {
                Some(Box::new(stream_network_analysis::HortonStreamOrder::new()))
            }
            "lengthofupstreamchannels" => Some(Box::new(
                stream_network_analysis::LengthOfUpstreamChannels::new(),
            )),
            "longprofile" => Some(Box::new(stream_network_analysis::LongProfile::new())),
            "longprofilefrompoints" => Some(Box::new(
                stream_network_analysis::LongProfileFromPoints::new(),
            )),
            "rasterizestreams" => Some(Box::new(stream_network_analysis::RasterizeStreams::new())),
            "rasterstreamstovector" => Some(Box::new(
                stream_network_analysis::RasterStreamsToVector::new(),
            )),
            "removeshortstreams" => {
                Some(Box::new(stream_network_analysis::RemoveShortStreams::new()))
            }
            "shrevestreammagnitude" => Some(Box::new(
                stream_network_analysis::ShreveStreamMagnitude::new(),
            )),
            "strahlerstreamorder" => {
                Some(Box::new(stream_network_analysis::StrahlerStreamOrder::new()))
            }
            "streamlinkclass" => Some(Box::new(stream_network_analysis::StreamLinkClass::new())),
            "streamlinkidentifier" => Some(Box::new(
                stream_network_analysis::StreamLinkIdentifier::new(),
            )),
            "streamlinklength" => Some(Box::new(stream_network_analysis::StreamLinkLength::new())),
            "streamlinkslope" => Some(Box::new(stream_network_analysis::StreamLinkSlope::new())),
            "streamslopecontinuous" => Some(Box::new(
                stream_network_analysis::StreamSlopeContinuous::new(),
            )),
            "topologicalstreamorder" => Some(Box::new(
                stream_network_analysis::TopologicalStreamOrder::new(),
            )),
            "tributaryidentifier" => {
                Some(Box::new(stream_network_analysis::TributaryIdentifier::new()))
            }

            // terrain_analysis
            "aspect" => Some(Box::new(terrain_analysis::Aspect::new())),
            "averagenormalvectorangulardeviation" => Some(Box::new(
                terrain_analysis::AverageNormalVectorAngularDeviation::new(),
            )),
            "circularvarianceofaspect" => {
                Some(Box::new(terrain_analysis::CircularVarianceOfAspect::new()))
            }
            "contoursfrompoints" => Some(Box::new(terrain_analysis::ContoursFromPoints::new())),
            "contoursfromraster" => Some(Box::new(terrain_analysis::ContoursFromRaster::new())),
            "devfrommeanelev" => Some(Box::new(terrain_analysis::DevFromMeanElev::new())),
            "difffrommeanelev" => Some(Box::new(terrain_analysis::DiffFromMeanElev::new())),
            "directionalrelief" => Some(Box::new(terrain_analysis::DirectionalRelief::new())),
            "downslopeindex" => Some(Box::new(terrain_analysis::DownslopeIndex::new())),
            "edgedensity" => Some(Box::new(terrain_analysis::EdgeDensity::new())),
            "elevabovepit" => Some(Box::new(terrain_analysis::ElevAbovePit::new())),
            "elevpercentile" => Some(Box::new(terrain_analysis::ElevPercentile::new())),
            "elevrelativetominmax" => Some(Box::new(terrain_analysis::ElevRelativeToMinMax::new())),
            "elevrelativetowatershedminmax" => Some(Box::new(
                terrain_analysis::ElevRelativeToWatershedMinMax::new(),
            )),
            "embankmentmapping" => Some(Box::new(terrain_analysis::EmbankmentMapping::new())),
            "featurepreservingsmoothing" => {
                Some(Box::new(terrain_analysis::FeaturePreservingSmoothing::new()))
            }
            "fetchanalysis" => Some(Box::new(terrain_analysis::FetchAnalysis::new())),
            "fillmissingdata" => Some(Box::new(terrain_analysis::FillMissingData::new())),
            "findridges" => Some(Box::new(terrain_analysis::FindRidges::new())),
            "gaussiancurvature" => Some(Box::new(terrain_analysis::GaussianCurvature::new())),
            "geomorphons" => Some(Box::new(terrain_analysis::Geomorphons::new())),
            "hillshade" => Some(Box::new(terrain_analysis::Hillshade::new())),
            "horizonangle" => Some(Box::new(terrain_analysis::HorizonAngle::new())),
            "hypsometricanalysis" => Some(Box::new(terrain_analysis::HypsometricAnalysis::new())),
            "hypsometricallytintedhillshade" => Some(Box::new(
                terrain_analysis::HypsometricallyTintedHillshade::new(),
            )),
            "mapoffterrainobjects" => Some(Box::new(terrain_analysis::MapOffTerrainObjects::new())),
            "maxanisotropydev" => Some(Box::new(terrain_analysis::MaxAnisotropyDev::new())),
            "maxanisotropydevsignature" => {
                Some(Box::new(terrain_analysis::MaxAnisotropyDevSignature::new()))
            }
            "maxbranchlength" => Some(Box::new(terrain_analysis::MaxBranchLength::new())),
            "maxdifferencefrommean" => {
                Some(Box::new(terrain_analysis::MaxDifferenceFromMean::new()))
            }
            "maxdownslopeelevchange" => {
                Some(Box::new(terrain_analysis::MaxDownslopeElevChange::new()))
            }
            "maxelevationdeviation" => {
                Some(Box::new(terrain_analysis::MaxElevationDeviation::new()))
            }
            "maxelevdevsignature" => Some(Box::new(terrain_analysis::MaxElevDevSignature::new())),
            "maxupslopeelevchange" => Some(Box::new(terrain_analysis::MaxUpslopeElevChange::new())),
            "maximalcurvature" => Some(Box::new(terrain_analysis::MaximalCurvature::new())),
            "meancurvature" => Some(Box::new(terrain_analysis::MeanCurvature::new())),
            "mindownslopeelevchange" => {
                Some(Box::new(terrain_analysis::MinDownslopeElevChange::new()))
            }
            "minimalcurvature" => Some(Box::new(terrain_analysis::MinimalCurvature::new())),
            "multidirectionalhillshade" => {
                Some(Box::new(terrain_analysis::MultidirectionalHillshade::new()))
            }
            "multiscaleelevationpercentile" => Some(Box::new(
                terrain_analysis::MultiscaleElevationPercentile::new(),
            )),
            "multiscaleroughness" => Some(Box::new(terrain_analysis::MultiscaleRoughness::new())),
            "multiscalestddevnormals" => {
                Some(Box::new(terrain_analysis::MultiscaleStdDevNormals::new()))
            }
            "multiscalestddevnormalssignature" => Some(Box::new(
                terrain_analysis::MultiscaleStdDevNormalsSignature::new(),
            )),
            "multiscaleroughnesssignature" => Some(Box::new(
                terrain_analysis::MultiscaleRoughnessSignature::new(),
            )),
            "multiscaletopographicpositionimage" => Some(Box::new(
                terrain_analysis::MultiscaleTopographicPositionImage::new(),
            )),
            "numdownslopeneighbours" => {
                Some(Box::new(terrain_analysis::NumDownslopeNeighbours::new()))
            }
            "numupslopeneighbours" => Some(Box::new(terrain_analysis::NumUpslopeNeighbours::new())),
            "pennocklandformclass" => Some(Box::new(terrain_analysis::PennockLandformClass::new())),
            "percentelevrange" => Some(Box::new(terrain_analysis::PercentElevRange::new())),
            "plancurvature" => Some(Box::new(terrain_analysis::PlanCurvature::new())),
            "profilecurvature" => Some(Box::new(terrain_analysis::ProfileCurvature::new())),
            "profile" => Some(Box::new(terrain_analysis::Profile::new())),
            "relativeaspect" => Some(Box::new(terrain_analysis::RelativeAspect::new())),
            "streampowerindex" => Some(Box::new(terrain_analysis::StreamPowerIndex::new())),
            "relativetopographicposition" => Some(Box::new(
                terrain_analysis::RelativeTopographicPosition::new(),
            )),
            "removeoffterrainobjects" => {
                Some(Box::new(terrain_analysis::RemoveOffTerrainObjects::new()))
            }
            "ruggednessindex" => Some(Box::new(terrain_analysis::RuggednessIndex::new())),
            // "segmentterrain" => Some(Box::new(terrain_analysis::SegmentTerrain::new())),
            "timeindaylight" => Some(Box::new(terrain_analysis::TimeInDaylight::new())),
            "sedimenttransportindex" => {
                Some(Box::new(terrain_analysis::SedimentTransportIndex::new()))
            }
            "slope" => Some(Box::new(terrain_analysis::Slope::new())),
            "slopevselevationplot" => Some(Box::new(terrain_analysis::SlopeVsElevationPlot::new())),
            "sphericalstddevofnormals" => {
                Some(Box::new(terrain_analysis::SphericalStdDevOfNormals::new()))
            }
            "standarddeviationofslope" => {
                Some(Box::new(terrain_analysis::StandardDeviationOfSlope::new()))
            }
            "surfacearearatio" => Some(Box::new(terrain_analysis::SurfaceAreaRatio::new())),
            "tangentialcurvature" => Some(Box::new(terrain_analysis::TangentialCurvature::new())),
            "totalcurvature" => Some(Box::new(terrain_analysis::TotalCurvature::new())),
            "viewshed" => Some(Box::new(terrain_analysis::Viewshed::new())),
            "visibilityindex" => Some(Box::new(terrain_analysis::VisibilityIndex::new())),
            "wetnessindex" => Some(Box::new(terrain_analysis::WetnessIndex::new())),

            _ => None,
        }
    }

    fn get_plugin_list(&self) -> Result<HashMap<String, serde_json::Value>, Error> {
        // let exe_path = std::env::current_dir()?.to_str().unwrap_or("No exe path found.").to_string();
        let mut dir = env::current_exe()?;
        dir.pop();
        dir.push("plugins");
        let plugin_directory = dir.to_str().unwrap_or("No exe path found.").to_string();
        // let plugin_directory = exe_path + &path::MAIN_SEPARATOR.to_string() + "plugins";
        // println!("{}", plugin_directory);
        // let mut plugin_names = vec![];
        let mut plugins = HashMap::new();
        if std::path::Path::new(&plugin_directory).is_dir() {
            for entry in std::fs::read_dir(plugin_directory.clone())? {
                let s = entry?
                    .path()
                    .into_os_string()
                    .to_str()
                    .expect("Error reading path string")
                    .to_string();
                if s.to_lowercase().ends_with(".json") && !s.to_lowercase().contains("._") { // no hidden files!
                    let contents = fs::read_to_string(s).expect("Something went wrong reading the file");
                    let mut v: serde_json::Value = serde_json::from_str(&contents)?;
                    v["plugin_directory"] = serde_json::json!(plugin_directory);
                    // println!("{}", v);
                    // plugin_names.push(contents);
                    plugins.insert(String::from(v["tool_name"].as_str().unwrap_or("no toolName").to_lowercase()), v);
                }
            }
        }

        Ok(plugins)
    }

    pub fn run_tool(&self, tool_name: String, args: Vec<String>) -> Result<(), Error> {
        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => return tool.run(args, &self.working_dir, self.verbose),
            None => {
                // Check the 'plugins' folder to see if the tool is in the Enterprise plugins.
                // if yes, then run it.
                let plugin_list = self.get_plugin_list()?;
                if plugin_list.contains_key(&tool_name.to_lowercase()) {
                    let plugin_data = plugin_list.get(&tool_name.to_lowercase()).expect(&format!("Unrecognized plugin name {}.", tool_name));
                    let ext = if cfg!(target_os = "windows") {
                        ".exe"
                    } else {
                        ""
                    };
                    // environment flags should be consumed by plugins using the settings.json file instead.
                    let mut args2 = vec![];
                    for a in 0..args.len() {
                        if args[a] != "-v" && args[a] != "--compress_rasters" {
                            args2.push(args[a].clone());
                        }
                    }
                    let exe = format!("{}{}{}{}", 
                        plugin_data["plugin_directory"]
                        .as_str()
                        .expect("Error: plugin executable name is unspecified."),
                        &path::MAIN_SEPARATOR.to_string(),
                        plugin_data["exe"]
                        .as_str()
                        .expect("Error: plugin executable name is unspecified.")
                        .replace("\"", ""),
                        ext
                    );
                    let mut subcommand = vec!["run".to_string()];
                    subcommand.extend_from_slice(&args);
                    let mut child = Command::new(exe)
                        .arg("run")
                        .args(&args2)
                        .spawn()
                        .expect("failed to execute process");

                    let ecode = child.wait()
                        .expect("failed to wait on child");
                    
                    if !ecode.success() {
                        println!("Failure to run plugin subprocess.");
                    }
                } else {
                    // We couldn't find an executable file for the tool, but still check to see if it's 
                    // one of the extension plugins. If it is, issue a 'need valid license' warning. If 
                    // not, then issue an unrecognized tool error.
                    let plugin_names = vec![
                        "accumulationcurvature",
                        "assessroute",
                        "cannyedgedetection", 
                        "curvedness",
                        "dbscan",
                        "differencecurvature",
                        "evaluatetrainingsites", 
                        "filterlidar",
                        "fix_danglingarcs",
                        "generalizeclassifiedraster",
                        "generalizewithsimilarity",
                        "generatingfunction",
                        "horizontalexcesscurvature",
                        "hydrologicconnectivity",
                        "imagesegmentation",
                        "imageslider",
                        "inversepca", 
                        "knn_classification",
                        "knn_regression",
                        "lastolaz",
                        "laztolas",
                        "lidarcontour",
                        "lidarpointreturnanalysis",
                        "lidarsibsoninterpolation", 
                        "lidarsortbytime", 
                        "localhypsometricanalysis",
                        "logistic_regression",
                        "lowpointsonheadwaterdivides",
                        "mindistclassification",
                        "modifylidar",
                        "openness",
                        "parallelepipedclassification",
                        "phicoefficient",
                        "random_forest_classification",
                        "random_forest_regression",
                        "reconcilemultipleheaders",
                        "recoverflightlineinfo",
                        "recreatepasslines",
                        "registerlicense",
                        "removefieldedgepoints",
                        "repairstreamvectortopology",
                        "ringcurvature",
                        "rotor",
                        "shadowanimation",
                        "shadowimage",
                        "slopevsaspectplot",
                        "smoothvegetationresidual",
                        "splitlidar",
                        "svm_classification",
                        "svm_regression",
                        "topographicpositionanimation",
                        "unsphericity",
                        "vectorstreamnetworkanalysis",
                        "verticalexcesscurvature",
                        "yieldfilter",
                        "yieldmap",
                        "yieldnormalization"
                    ];
                    if plugin_names.contains(&tool_name.to_lowercase().as_ref()) {
                        return Err(Error::new(
                            ErrorKind::NotFound,
                            format!("Invalid license: \nThis tool is part of a Whitebox extension product \nand there is a missing license. Please contact \nWhitebox Geospatial Inc. (support@whiteboxgeo.com) to obtain \na valid license key."),
                        ))
                    } else {
                        return Err(Error::new(
                            ErrorKind::NotFound,
                            format!("Unrecognized tool name {}.", tool_name),
                        ))
                    }
                }
                return Ok(())
            }
        }
    }

    pub fn tool_help(&self, tool_name: String) -> Result<(), Error> {
        if !tool_name.is_empty() {
            match self.get_tool(tool_name.as_ref()) {
                Some(tool) => println!("{}", get_help(tool)),
                None => {
                    let plugin_list = self.get_plugin_list()?;
                    if plugin_list.contains_key(&tool_name.to_lowercase()) {
                        let plugin_data = plugin_list.get(&tool_name.to_lowercase()).expect(&format!("Unrecognized plugin name {}.", tool_name));
                        // println!("{}", plugin_data["help"].as_str().expect("Cannot locate plugin tool help."))

                        let tool_name = plugin_data["tool_name"].as_str().expect("Cannot locate plugin tool name.");
                        let description = plugin_data["short_description"].as_str().expect("Cannot locate plugin tool description.");
                        let toolbox = plugin_data["toolbox"].as_str().expect("Cannot locate plugin toolbox.");
                        let a = plugin_data["parameters"].as_array().unwrap();
                        let mut p = String::new();
                        p.push_str("Flag               Description\n");
                        p.push_str("-----------------  -----------\n");
                        for d in a {
                            let mut s = String::new();
                            for f in d["flags"].as_array().unwrap() {
                                s.push_str(&format!("{}, ", f.as_str().unwrap()));
                            }
                            p.push_str(&format!(
                                "{:width$} {}\n",
                                s.trim().trim_matches(','),
                                d["description"].as_str().unwrap(),
                                width = 18
                            ));
                        }
                        let tmp_example = plugin_data["example"].as_str().unwrap_or("Example not located.");

                        let sep: String = std::path::MAIN_SEPARATOR.to_string();
                        // let k = format!("{}", std::env::current_dir().unwrap().display());
                        let mut dir = env::current_exe()?;
                        dir.pop();
                        let exe_directory = dir.to_str().unwrap_or("No exe path found.").to_string();
                        let e = format!("{}", std::env::current_exe().unwrap().display());
                        let mut short_exe = e
                            .replace(&exe_directory, "")
                            .replace(".exe", "")
                            .replace(".", "")
                            .replace(&sep, "");
                        if e.contains(".exe") {
                            short_exe += ".exe";
                        }

                        let example = &tmp_example.replace("*", &sep).replace("EXE_NAME", &short_exe);

                        let s: String;
                        if example.len() <= 1 {
                            s = format!(
            "{}

Description:\n{}
Toolbox: {}
Parameters:\n
{}
",
                            tool_name, description, toolbox, p
                        );
                    } else {
                s = format!(
            "{}
Description:\n{}
Toolbox: {}
Parameters:\n
{}

Example usage:
{}
",
                                tool_name, description, toolbox, p, example
                            );
                        }
                        println!("{}", s);

                    } else {
                        return Err(Error::new(
                            ErrorKind::NotFound,
                            format!("Unrecognized tool name {}.", tool_name),
                        ))
                    }
                }
            }
        } else {
            let mut i = 1;
            for val in &self.tool_names {
                let tool = self.get_tool(&val).unwrap();
                println!("{}. {}\n", i, get_help(tool));
                i += 1;
            }
        }
        Ok(())
    }

    pub fn tool_license(&self, tool_name: String) -> Result<(), Error> {
        match self.get_tool(tool_name.as_ref()) {
            Some(_tool) => println!("MIT"),
            None => {
                let plugin_list = self.get_plugin_list()?;
                if plugin_list.contains_key(&tool_name.to_lowercase()) {
                    let plugin_data = plugin_list.get(&tool_name.to_lowercase()).expect(&format!("Unrecognized plugin name {}.", tool_name));
                    println!("{}", plugin_data["license"].as_str().expect("Cannot locate plugin tool license."));
                } else {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        format!("Unrecognized tool name {}.", tool_name),
                    ))
                }
            }
        }
        Ok(())
    }

    pub fn tool_parameters(&self, tool_name: String) -> Result<(), Error> {
        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => println!("{}", tool.get_tool_parameters()),
            None => {
                // println!("I'm here {}", tool_name);
                let plugin_list = self.get_plugin_list()?;
                if plugin_list.contains_key(&tool_name.to_lowercase()) {
                    let plugin_data = plugin_list.get(&tool_name.to_lowercase()).expect(&format!("Unrecognized plugin name {}.", tool_name));
                    // println!("{:?}", plugin_data);
                    println!("{}", plugin_data);
                } else {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        format!("Unrecognized tool name {}.", tool_name),
                    ))
                }
            }
        }
        Ok(())
    }

    pub fn toolbox(&self, tool_name: String) -> Result<(), Error> {
        if !tool_name.is_empty() {
            match self.get_tool(tool_name.as_ref()) {
                Some(tool) => println!("{}", tool.get_toolbox()),
                None => {
                    let plugin_list = self.get_plugin_list()?;
                    if plugin_list.contains_key(&tool_name.to_lowercase()) {
                        let plugin_data = plugin_list.get(&tool_name.to_lowercase()).expect(&format!("Unrecognized plugin name {}.", tool_name));
                        let toolbox = plugin_data["toolbox"].as_str().unwrap_or("Toolbox name not found.");
                        println!("{}", toolbox);
                    } else {
                        return Err(Error::new(
                            ErrorKind::NotFound,
                            format!("Unrecognized tool name {}.", tool_name),
                        ))
                    }
                    // for (_key, plugin_data) in &plugin_list {
                    //     let tool = plugin_data["tool_name"].as_str().unwrap_or("Tool name not found.");
                    //     if tool == tool_name {
                    //         let toolbox = plugin_data["toolbox"].as_str().unwrap_or("Toolbox name not found.");
                    //         println!("{}", toolbox);
                    //     }
                    // }
                    // return Err(Error::new(
                    //     ErrorKind::NotFound,
                    //     format!("Unrecognized tool name {}.", tool_name),
                    // ))
                }
            }
        } else {
            let mut tool_details: Vec<(String, String)> = Vec::new();
            for val in &self.tool_names {
                let tool = self.get_tool(&val).unwrap();
                let toolbox = tool.get_toolbox();
                // println!("{}: {}\n", val, toolbox);
                tool_details.push((val.to_string(), toolbox.to_string()));
            }

            let plugin_list = self.get_plugin_list()?;
            for (_key, plugin_data) in &plugin_list {
                let tool = plugin_data["tool_name"].as_str().unwrap_or("Tool name not found.");
                let toolbox = plugin_data["toolbox"].as_str().unwrap_or("Toolbox name not found.");
                // println!("{}: {}\n", tool, toolbox);
                tool_details.push((tool.to_string(), toolbox.to_string()));
            }

            tool_details.sort();
            for i in 0..tool_details.len() {
                println!("{}: {}", tool_details[i].0, tool_details[i].1);
            }
        }
        Ok(())
    }

    pub fn list_tools(&self) {
        let mut tool_details: Vec<(String, String)> = Vec::new();

        for val in &self.tool_names {
            let tool = self
                .get_tool(&val)
                .expect(&format!("Unrecognized tool name {}.", val));
            tool_details.push(get_name_and_description(tool));
        }

        let plugin_list = self.get_plugin_list().unwrap();
        for (_key, plugin_data) in &plugin_list {
            let tool = plugin_data["tool_name"].as_str().unwrap_or("Tool name not found.");
            let description = plugin_data["short_description"].as_str().unwrap_or("Tool description name not found.");
            tool_details.push((tool.to_string(), description.to_string()));
        }

        tool_details.sort();

        let mut ret = format!("All {} Available Tools:\n", tool_details.len());
        for i in 0..tool_details.len() {
            ret.push_str(&format!("{}: {}\n\n", tool_details[i].0, tool_details[i].1));
        }
        println!("{}", ret);
    }

    pub fn list_tools_with_keywords(&self, keywords: Vec<String>) {
        let mut tool_details: Vec<(String, String)> = Vec::new();
        for val in &self.tool_names {
            let tool = self
                .get_tool(&val)
                .expect(&format!("Unrecognized tool name {}.", val));
            let toolbox = tool.get_toolbox();
            let (nm, des) = get_name_and_description(tool);
            for kw in &keywords {
                if nm.to_lowercase().contains(&(kw.to_lowercase()))
                    || des.to_lowercase().contains(&(kw.to_lowercase()))
                    || toolbox.to_lowercase().contains(&(kw.to_lowercase()))
                {
                    tool_details.push(get_name_and_description(
                        self.get_tool(&val)
                            .expect(&format!("Unrecognized tool name {}.", val)),
                    ));
                    break;
                }
            }
        }

        let plugin_list = self.get_plugin_list().unwrap();
        for (_key, plugin_data) in &plugin_list {
            let nm = plugin_data["tool_name"].as_str().unwrap_or("Tool name not found.");
            let des = plugin_data["short_description"].as_str().unwrap_or("Tool description name not found.");
            for kw in &keywords {
                if nm.to_lowercase().contains(&(kw.to_lowercase()))
                    || des.to_lowercase().contains(&(kw.to_lowercase()))
                {
                    tool_details.push((nm.to_string(), des.to_string()));
                    break;
                }
            }
        }

        let mut ret = format!("All {} Tools containing keywords:\n", tool_details.len());
        for i in 0..tool_details.len() {
            ret.push_str(&format!("{}: {}\n\n", tool_details[i].0, tool_details[i].1));
        }

        println!("{}", ret);
    }

    pub fn get_tool_source_code(&self, tool_name: String) -> Result<(), Error> {
        let repo = String::from("https://github.com/jblindsay/whitebox-tools/blob/master/");
        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => println!("{}{}", repo, tool.get_source_file()),
            None => {
                let plugin_list = self.get_plugin_list()?;
                let license: String;
                if plugin_list.contains_key(&tool_name.to_lowercase()) {
                    let plugin_data = plugin_list.get(&tool_name.to_lowercase()).expect(&format!("Unrecognized plugin name {}.", tool_name));
                    license = plugin_data["license"].as_str().expect("Cannot locate plugin tool license.").to_lowercase();
                } else {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        format!("Unrecognized tool name {}.", tool_name),
                    ))
                }
                // let license = self.tool_license(tool_name.clone()).to_lowercase();
                if !license.contains("proprietary") {
                    println!("https://github.com/jblindsay/whitebox-tools/blob/master/{}", tool_name);
                } else {
                    println!("Source code is unavailable due to proprietary license.");
                }
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Unrecognized tool name {}.", tool_name),
                ))
            }
        }

        Ok(())
    }
}

pub trait WhiteboxTool {
    fn get_tool_name(&self) -> String;
    fn get_tool_description(&self) -> String;
    fn get_tool_parameters(&self) -> String;
    fn get_example_usage(&self) -> String;
    fn get_toolbox(&self) -> String;
    fn get_source_file(&self) -> String;
    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error>;
}

fn get_help<'a>(wt: Box<dyn WhiteboxTool + 'a>) -> String {
    let tool_name = wt.get_tool_name();
    let description = wt.get_tool_description();
    let parameters = wt.get_tool_parameters();
    let toolbox = wt.get_toolbox();
    let o: serde_json::Value = serde_json::from_str(&parameters).unwrap();
    let a = o["parameters"].as_array().unwrap();
    let mut p = String::new();
    p.push_str("Flag               Description\n");
    p.push_str("-----------------  -----------\n");
    for d in a {
        let mut s = String::new();
        for f in d["flags"].as_array().unwrap() {
            s.push_str(&format!("{}, ", f.as_str().unwrap()));
        }
        p.push_str(&format!(
            "{:width$} {}\n",
            s.trim().trim_matches(','),
            d["description"].as_str().unwrap(),
            width = 18
        ));
    }
    let example = wt.get_example_usage();
    let s: String;
    if example.len() <= 1 {
        s = format!(
            "{}

Description:\n{}
Toolbox: {}
Parameters:\n
{}
",
            tool_name, description, toolbox, p
        );
    } else {
        s = format!(
            "{}
Description:\n{}
Toolbox: {}
Parameters:\n
{}

Example usage:
{}
",
            tool_name, description, toolbox, p, example
        );
    }
    s
}

fn get_name_and_description<'a>(wt: Box<dyn WhiteboxTool + 'a>) -> (String, String) {
    (wt.get_tool_name(), wt.get_tool_description())
}

#[derive(Serialize, Deserialize, Debug)]
struct ToolParameter {
    name: String,
    flags: Vec<String>,
    description: String,
    parameter_type: ParameterType,
    default_value: Option<String>,
    optional: bool,
}

impl ToolParameter {
    pub fn to_string(&self) -> String {
        let v = match serde_json::to_string(&self) {
            Ok(json_str) => json_str,
            Err(err) => format!("{:?}", err),
        };
        v
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum ParameterType {
    Boolean,
    String,
    StringList,
    Integer,
    Float,
    VectorAttributeField(AttributeType, String),
    StringOrNumber,
    ExistingFile(ParameterFileType),
    ExistingFileOrFloat(ParameterFileType),
    NewFile(ParameterFileType),
    FileList(ParameterFileType),
    Directory,
    OptionList(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug)]
enum ParameterFileType {
    Any,
    Lidar,
    Raster,
    RasterAndVector(VectorGeometryType),
    Vector(VectorGeometryType),
    Text,
    Html,
    Csv,
}

#[derive(Serialize, Deserialize, Debug)]
enum VectorGeometryType {
    Any,
    Point,
    Line,
    Polygon,
    LineOrPolygon,
}

#[derive(Serialize, Deserialize, Debug)]
enum AttributeType {
    Any,
    Integer,
    Float,
    Number,
    Text,
    Boolean,
    Date,
}
