pub mod data_tools;
pub mod gis_analysis;
pub mod hydro_analysis;
pub mod image_analysis;
pub mod lidar_analysis;
pub mod math_stat_analysis;
pub mod stream_network_analysis;
pub mod terrain_analysis;

use serde_json;
use std::io::{Error, ErrorKind};
use tools;

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
        tool_names.push("ConvertNodataToZero".to_string());
        tool_names.push("ConvertRasterFormat".to_string());
        tool_names.push("ExportTableToCsv".to_string());
        tool_names.push("IdwInterpolation".to_string());
        tool_names.push("NewRasterFromBase".to_string());
        tool_names.push("PolygonsToLines".to_string());
        tool_names.push("PrintGeoTiffTags".to_string());
        tool_names.push("ReinitializeAttributeTable".to_string());
        tool_names.push("SetNodataValue".to_string());
        tool_names.push("VectorLinesToRaster".to_string());
        tool_names.push("VectorPointsToRaster".to_string());
        tool_names.push("VectorPolygonsToRaster".to_string());

        // gis_analysis
        tool_names.push("AggregateRaster".to_string());
        tool_names.push("AverageOverlay".to_string());
        tool_names.push("BufferRaster".to_string());
        tool_names.push("Centroid".to_string());
        tool_names.push("ClipRasterToPolygon".to_string());
        tool_names.push("Clump".to_string());
        tool_names.push("CountIf".to_string());
        tool_names.push("CostAllocation".to_string());
        tool_names.push("CostDistance".to_string());
        tool_names.push("CostPathway".to_string());
        tool_names.push("CreatePlane".to_string());
        tool_names.push("EdgeProportion".to_string());
        tool_names.push("ErasePolygonFromRaster".to_string());
        tool_names.push("EuclideanAllocation".to_string());
        tool_names.push("EuclideanDistance".to_string());
        tool_names.push("ExtractNodes".to_string());
        tool_names.push("ExtractRasterValuesAtPoints".to_string());
        tool_names.push("FindLowestOrHighestPoints".to_string());
        tool_names.push("FindPatchOrClassEdgeCells".to_string());
        tool_names.push("HighestPosition".to_string());
        tool_names.push("LowestPosition".to_string());
        tool_names.push("MaxAbsoluteOverlay".to_string());
        tool_names.push("MaxOverlay".to_string());
        tool_names.push("MinAbsoluteOverlay".to_string());
        tool_names.push("MinimumBoundingBox".to_string());
        tool_names.push("MinimumConvexHull".to_string());
        tool_names.push("MinOverlay".to_string());
        tool_names.push("PercentEqualTo".to_string());
        tool_names.push("PercentGreaterThan".to_string());
        tool_names.push("PercentLessThan".to_string());
        tool_names.push("PickFromList".to_string());
        tool_names.push("PolygonLongAxis".to_string());
        tool_names.push("PolygonShortAxis".to_string());
        tool_names.push("RadiusOfGyration".to_string());
        tool_names.push("RasterCellAssignment".to_string());
        tool_names.push("Reclass".to_string());
        tool_names.push("ReclassEqualInterval".to_string());
        tool_names.push("ReclassFromFile".to_string());
        tool_names.push("WeightedOverlay".to_string());
        tool_names.push("WeightedSum".to_string());

        // hydro_analysis
        tool_names.push("AverageFlowpathSlope".to_string());
        tool_names.push("AverageUpslopeFlowpathLength".to_string());
        tool_names.push("Basins".to_string());
        tool_names.push("BreachDepressions".to_string());
        tool_names.push("BreachSingleCellPits".to_string());
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
        tool_names.push("FillSingleCellPits".to_string());
        tool_names.push("FindNoFlowCells".to_string());
        tool_names.push("FindParallelFlow".to_string());
        tool_names.push("FlattenLakes".to_string());
        tool_names.push("FloodOrder".to_string());
        tool_names.push("FlowAccumulationFullWorkflow".to_string());
        tool_names.push("FlowLengthDiff".to_string());
        tool_names.push("Hillslopes".to_string());
        tool_names.push("ImpoundmentIndex".to_string());
        tool_names.push("Isobasins".to_string());
        tool_names.push("JensonSnapPourPoints".to_string());
        tool_names.push("MaxUpslopeFlowpathLength".to_string());
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
        tool_names.push("LeeFilter".to_string());
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
        tool_names.push("NormalizedDifferenceVegetationIndex".to_string());
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
        tool_names.push("BlockMaximum".to_string());
        tool_names.push("BlockMinimum".to_string());
        tool_names.push("ClassifyOverlapPoints".to_string());
        tool_names.push("ClipLidarToPolygon".to_string());
        tool_names.push("ErasePolygonFromLidar".to_string());
        tool_names.push("FilterLidarScanAngles".to_string());
        tool_names.push("FindFlightlineEdgePoints".to_string());
        tool_names.push("FlightlineOverlap".to_string());
        tool_names.push("LasToAscii".to_string());
        tool_names.push("LasToMultipointShapefile".to_string());
        tool_names.push("LidarColourize".to_string());
        tool_names.push("LidarElevationSlice".to_string());
        tool_names.push("LidarGroundPointFilter".to_string());
        tool_names.push("LidarHillshade".to_string());
        tool_names.push("LidarHistogram".to_string());
        tool_names.push("LidarIdwInterpolation".to_string());
        tool_names.push("LidarInfo".to_string());
        tool_names.push("LidarJoin".to_string());
        tool_names.push("LidarKappaIndex".to_string());
        tool_names.push("LidarNearestNeighbourGridding".to_string());
        tool_names.push("LidarPointDensity".to_string());
        tool_names.push("LidarPointStats".to_string());
        tool_names.push("LidarRemoveDuplicates".to_string());
        tool_names.push("LidarRemoveOutliers".to_string());
        tool_names.push("LidarSegmentation".to_string());
        tool_names.push("LidarSegmentationBasedFilter".to_string());
        tool_names.push("LidarThin".to_string());
        tool_names.push("LidarThinHighDensity".to_string());
        tool_names.push("LidarTile".to_string());
        tool_names.push("LidarTileFootprint".to_string());
        tool_names.push("LidarTophatTransform".to_string());
        tool_names.push("NormalVectors".to_string());
        tool_names.push("SelectTilesByPolygon".to_string());

        // mathematical and statistical_analysis
        tool_names.push("AbsoluteValue".to_string());
        tool_names.push("Add".to_string());
        tool_names.push("And".to_string());
        tool_names.push("Anova".to_string());
        tool_names.push("ArcCos".to_string());
        tool_names.push("ArcSin".to_string());
        tool_names.push("ArcTan".to_string());
        tool_names.push("Atan2".to_string());
        tool_names.push("AttributeCorrelation".to_string());
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
        tool_names.push("ExtractRasterStatistics".to_string());
        tool_names.push("Floor".to_string());
        tool_names.push("GreaterThan".to_string());
        tool_names.push("ImageAutocorrelation".to_string());
        tool_names.push("ImageCorrelation".to_string());
        tool_names.push("ImageRegression".to_string());
        tool_names.push("Increment".to_string());
        tool_names.push("InPlaceAdd".to_string());
        tool_names.push("InPlaceDivide".to_string());
        tool_names.push("InPlaceMultiply".to_string());
        tool_names.push("InPlaceSubtract".to_string());
        tool_names.push("IntegerDivision".to_string());
        tool_names.push("IsNoData".to_string());
        tool_names.push("KappaIndex".to_string());
        tool_names.push("KSTestForNormality".to_string());
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
        tool_names.push("DevFromMeanElev".to_string());
        tool_names.push("DiffFromMeanElev".to_string());
        tool_names.push("DirectionalRelief".to_string());
        tool_names.push("DownslopeIndex".to_string());
        tool_names.push("ElevAbovePit".to_string());
        tool_names.push("ElevPercentile".to_string());
        tool_names.push("ElevRelativeToMinMax".to_string());
        tool_names.push("ElevRelativeToWatershedMinMax".to_string());
        tool_names.push("FeaturePreservingDenoise".to_string());
        tool_names.push("FeaturePreservingDenoiseWithExclusions".to_string());
        tool_names.push("FetchAnalysis".to_string());
        tool_names.push("FillMissingData".to_string());
        tool_names.push("FindRidges".to_string());
        tool_names.push("Hillshade".to_string());
        tool_names.push("HorizonAngle".to_string());
        tool_names.push("HypsometricAnalysis".to_string());
        tool_names.push("MaxAnisotropyDev".to_string());
        tool_names.push("MaxAnisotropyDevSignature".to_string());
        tool_names.push("MaxBranchLength".to_string());
        tool_names.push("MaxDifferenceFromMean".to_string());
        tool_names.push("MaxDownslopeElevChange".to_string());
        tool_names.push("MaxElevDevSignature".to_string());
        tool_names.push("MaxElevationDeviation".to_string());
        tool_names.push("MinDownslopeElevChange".to_string());
        tool_names.push("MultiscaleRoughness".to_string());
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
        tool_names.push("RelativeStreamPowerIndex".to_string());
        tool_names.push("RelativeTopographicPosition".to_string());
        tool_names.push("RemoveOffTerrainObjects".to_string());
        tool_names.push("RuggednessIndex".to_string());
        tool_names.push("SedimentTransportIndex".to_string());
        tool_names.push("Slope".to_string());
        tool_names.push("SlopeVsElevationPlot".to_string());
        tool_names.push("StandardDeviationOfSlope".to_string());
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

    fn get_tool(&self, tool_name: &str) -> Option<Box<WhiteboxTool + 'static>> {
        match tool_name.to_lowercase().replace("_", "").as_ref() {
            // data_tools
            "convertnodatatozero" => Some(Box::new(tools::data_tools::ConvertNodataToZero::new())),
            "convertrasterformat" => Some(Box::new(tools::data_tools::ConvertRasterFormat::new())),
            "exporttabletocsv" => Some(Box::new(tools::data_tools::ExportTableToCsv::new())),
            "idwinterpolation" => Some(Box::new(tools::data_tools::IdwInterpolation::new())),
            "newrasterfrombase" => Some(Box::new(tools::data_tools::NewRasterFromBase::new())),
            "polygonstolines" => Some(Box::new(tools::data_tools::PolygonsToLines::new())),
            "printgeotifftags" => Some(Box::new(tools::data_tools::PrintGeoTiffTags::new())),
            "reinitializeattributetable" => Some(Box::new(
                tools::data_tools::ReinitializeAttributeTable::new(),
            )),
            "setnodatavalue" => Some(Box::new(tools::data_tools::SetNodataValue::new())),
            "vectorlinestoraster" => Some(Box::new(tools::data_tools::VectorLinesToRaster::new())),
            "vectorpointstoraster" => {
                Some(Box::new(tools::data_tools::VectorPointsToRaster::new()))
            }
            "vectorpolygonstoraster" => {
                Some(Box::new(tools::data_tools::VectorPolygonsToRaster::new()))
            }

            // gis_analysis
            "aggregateraster" => Some(Box::new(tools::gis_analysis::AggregateRaster::new())),
            "averageoverlay" => Some(Box::new(tools::gis_analysis::AverageOverlay::new())),
            "bufferraster" => Some(Box::new(tools::gis_analysis::BufferRaster::new())),
            "centroid" => Some(Box::new(tools::gis_analysis::Centroid::new())),
            "cliprastertopolygon" => {
                Some(Box::new(tools::gis_analysis::ClipRasterToPolygon::new()))
            }
            "clump" => Some(Box::new(tools::gis_analysis::Clump::new())),
            "countif" => Some(Box::new(tools::gis_analysis::CountIf::new())),
            "costallocation" => Some(Box::new(tools::gis_analysis::CostAllocation::new())),
            "costdistance" => Some(Box::new(tools::gis_analysis::CostDistance::new())),
            "costpathway" => Some(Box::new(tools::gis_analysis::CostPathway::new())),
            "createplane" => Some(Box::new(tools::gis_analysis::CreatePlane::new())),
            "edgeproportion" => Some(Box::new(tools::gis_analysis::EdgeProportion::new())),
            "erasepolygonfromraster" => {
                Some(Box::new(tools::gis_analysis::ErasePolygonFromRaster::new()))
            }
            "euclideanallocation" => {
                Some(Box::new(tools::gis_analysis::EuclideanAllocation::new()))
            }
            "euclideandistance" => Some(Box::new(tools::gis_analysis::EuclideanDistance::new())),
            "extractnodes" => Some(Box::new(tools::gis_analysis::ExtractNodes::new())),
            "extractrastervaluesatpoints" => Some(Box::new(
                tools::gis_analysis::ExtractRasterValuesAtPoints::new(),
            )),
            "findlowestorhighestpoints" => Some(Box::new(
                tools::gis_analysis::FindLowestOrHighestPoints::new(),
            )),
            "findpatchorclassedgecells" => Some(Box::new(
                tools::gis_analysis::FindPatchOrClassEdgeCells::new(),
            )),
            "highestposition" => Some(Box::new(tools::gis_analysis::HighestPosition::new())),
            "lowestposition" => Some(Box::new(tools::gis_analysis::LowestPosition::new())),
            "maxabsoluteoverlay" => Some(Box::new(tools::gis_analysis::MaxAbsoluteOverlay::new())),
            "maxoverlay" => Some(Box::new(tools::gis_analysis::MaxOverlay::new())),
            "minabsoluteoverlay" => Some(Box::new(tools::gis_analysis::MinAbsoluteOverlay::new())),
            "minimumboundingbox" => Some(Box::new(tools::gis_analysis::MinimumBoundingBox::new())),
            "minimumconvexhull" => Some(Box::new(tools::gis_analysis::MinimumConvexHull::new())),
            "minoverlay" => Some(Box::new(tools::gis_analysis::MinOverlay::new())),
            "percentequalto" => Some(Box::new(tools::gis_analysis::PercentEqualTo::new())),
            "percentgreaterthan" => Some(Box::new(tools::gis_analysis::PercentGreaterThan::new())),
            "percentlessthan" => Some(Box::new(tools::gis_analysis::PercentLessThan::new())),
            "pickfromlist" => Some(Box::new(tools::gis_analysis::PickFromList::new())),
            "polygonlongaxis" => Some(Box::new(tools::gis_analysis::PolygonLongAxis::new())),
            "polygonshortaxis" => Some(Box::new(tools::gis_analysis::PolygonShortAxis::new())),
            "radiusofgyration" => Some(Box::new(tools::gis_analysis::RadiusOfGyration::new())),
            "rastercellassignment" => {
                Some(Box::new(tools::gis_analysis::RasterCellAssignment::new()))
            }
            "reclass" => Some(Box::new(tools::gis_analysis::Reclass::new())),
            "reclassequalinterval" => {
                Some(Box::new(tools::gis_analysis::ReclassEqualInterval::new()))
            }
            "reclassfromfile" => Some(Box::new(tools::gis_analysis::ReclassFromFile::new())),
            "weightedoverlay" => Some(Box::new(tools::gis_analysis::WeightedOverlay::new())),
            "weightedsum" => Some(Box::new(tools::gis_analysis::WeightedSum::new())),

            // hydro_analysis
            "averageflowpathslope" => {
                Some(Box::new(tools::hydro_analysis::AverageFlowpathSlope::new()))
            }
            "averageupslopeflowpathlength" => Some(Box::new(
                tools::hydro_analysis::AverageUpslopeFlowpathLength::new(),
            )),
            "basins" => Some(Box::new(tools::hydro_analysis::Basins::new())),
            "breachdepressions" => Some(Box::new(tools::hydro_analysis::BreachDepressions::new())),
            "breachsinglecellpits" => {
                Some(Box::new(tools::hydro_analysis::BreachSingleCellPits::new()))
            }
            "d8flowaccumulation" => {
                Some(Box::new(tools::hydro_analysis::D8FlowAccumulation::new()))
            }
            "d8massflux" => Some(Box::new(tools::hydro_analysis::D8MassFlux::new())),
            "d8pointer" => Some(Box::new(tools::hydro_analysis::D8Pointer::new())),
            "depthinsink" => Some(Box::new(tools::hydro_analysis::DepthInSink::new())),
            "dinfflowaccumulation" => {
                Some(Box::new(tools::hydro_analysis::DInfFlowAccumulation::new()))
            }
            "dinfmassflux" => Some(Box::new(tools::hydro_analysis::DInfMassFlux::new())),
            "dinfpointer" => Some(Box::new(tools::hydro_analysis::DInfPointer::new())),
            "downslopedistancetostream" => Some(Box::new(
                tools::hydro_analysis::DownslopeDistanceToStream::new(),
            )),
            "downslopeflowpathlength" => Some(Box::new(
                tools::hydro_analysis::DownslopeFlowpathLength::new(),
            )),
            "elevationabovestream" => {
                Some(Box::new(tools::hydro_analysis::ElevationAboveStream::new()))
            }
            "elevationabovestreameuclidean" => Some(Box::new(
                tools::hydro_analysis::ElevationAboveStreamEuclidean::new(),
            )),
            "fd8flowaccumulation" => {
                Some(Box::new(tools::hydro_analysis::FD8FlowAccumulation::new()))
            }
            "fd8pointer" => Some(Box::new(tools::hydro_analysis::FD8Pointer::new())),
            "fillburn" => Some(Box::new(tools::hydro_analysis::FillBurn::new())),
            "filldepressions" => Some(Box::new(tools::hydro_analysis::FillDepressions::new())),
            "fillsinglecellpits" => {
                Some(Box::new(tools::hydro_analysis::FillSingleCellPits::new()))
            }
            "findnoflowcells" => Some(Box::new(tools::hydro_analysis::FindNoFlowCells::new())),
            "findparallelflow" => Some(Box::new(tools::hydro_analysis::FindParallelFlow::new())),
            "flattenlakes" => Some(Box::new(tools::hydro_analysis::FlattenLakes::new())),
            "floodorder" => Some(Box::new(tools::hydro_analysis::FloodOrder::new())),
            "flowaccumulationfullworkflow" => Some(Box::new(
                tools::hydro_analysis::FlowAccumulationFullWorkflow::new(),
            )),
            "flowlengthdiff" => Some(Box::new(tools::hydro_analysis::FlowLengthDiff::new())),
            "hillslopes" => Some(Box::new(tools::hydro_analysis::Hillslopes::new())),
            "impoundmentindex" => Some(Box::new(tools::hydro_analysis::ImpoundmentIndex::new())),
            "isobasins" => Some(Box::new(tools::hydro_analysis::Isobasins::new())),
            "jensonsnappourpoints" => {
                Some(Box::new(tools::hydro_analysis::JensonSnapPourPoints::new()))
            }
            "maxupslopeflowpathlength" => Some(Box::new(
                tools::hydro_analysis::MaxUpslopeFlowpathLength::new(),
            )),
            "numinflowingneighbours" => Some(Box::new(
                tools::hydro_analysis::NumInflowingNeighbours::new(),
            )),
            "raisewalls" => Some(Box::new(tools::hydro_analysis::RaiseWalls::new())),
            "rho8pointer" => Some(Box::new(tools::hydro_analysis::Rho8Pointer::new())),
            "sink" => Some(Box::new(tools::hydro_analysis::Sink::new())),
            "snappourpoints" => Some(Box::new(tools::hydro_analysis::SnapPourPoints::new())),
            "stochasticdepressionanalysis" => Some(Box::new(
                tools::hydro_analysis::StochasticDepressionAnalysis::new(),
            )),
            "strahlerorderbasins" => {
                Some(Box::new(tools::hydro_analysis::StrahlerOrderBasins::new()))
            }
            "subbasins" => Some(Box::new(tools::hydro_analysis::Subbasins::new())),
            "tracedownslopeflowpaths" => Some(Box::new(
                tools::hydro_analysis::TraceDownslopeFlowpaths::new(),
            )),
            "unnestbasins" => Some(Box::new(tools::hydro_analysis::UnnestBasins::new())),
            "watershed" => Some(Box::new(tools::hydro_analysis::Watershed::new())),

            // image_analysis
            "adaptivefilter" => Some(Box::new(tools::image_analysis::AdaptiveFilter::new())),
            "balancecontrastenhancement" => Some(Box::new(
                tools::image_analysis::BalanceContrastEnhancement::new(),
            )),
            "bilateralfilter" => Some(Box::new(tools::image_analysis::BilateralFilter::new())),
            "changevectoranalysis" => {
                Some(Box::new(tools::image_analysis::ChangeVectorAnalysis::new()))
            }
            "closing" => Some(Box::new(tools::image_analysis::Closing::new())),
            "cornerdetection" => Some(Box::new(tools::image_analysis::CornerDetection::new())),
            "correctvignetting" => Some(Box::new(tools::image_analysis::CorrectVignetting::new())),
            "conservativesmoothingfilter" => Some(Box::new(
                tools::image_analysis::ConservativeSmoothingFilter::new(),
            )),
            "createcolourcomposite" => {
                Some(Box::new(tools::image_analysis::CreateColourComposite::new()))
            }
            "directdecorrelationstretch" => Some(Box::new(
                tools::image_analysis::DirectDecorrelationStretch::new(),
            )),
            "diversityfilter" => Some(Box::new(tools::image_analysis::DiversityFilter::new())),
            "diffofgaussianfilter" => {
                Some(Box::new(tools::image_analysis::DiffOfGaussianFilter::new()))
            }
            "edgepreservingmeanfilter" => Some(Box::new(
                tools::image_analysis::EdgePreservingMeanFilter::new(),
            )),
            "embossfilter" => Some(Box::new(tools::image_analysis::EmbossFilter::new())),
            "fastalmostgaussianfilter" => Some(Box::new(
                tools::image_analysis::FastAlmostGaussianFilter::new(),
            )),
            "flipimage" => Some(Box::new(tools::image_analysis::FlipImage::new())),
            "gammacorrection" => Some(Box::new(tools::image_analysis::GammaCorrection::new())),
            "gaussiancontraststretch" => Some(Box::new(
                tools::image_analysis::GaussianContrastStretch::new(),
            )),
            "gaussianfilter" => Some(Box::new(tools::image_analysis::GaussianFilter::new())),
            "highpassfilter" => Some(Box::new(tools::image_analysis::HighPassFilter::new())),
            "highpassmedianfilter" => {
                Some(Box::new(tools::image_analysis::HighPassMedianFilter::new()))
            }
            "histogramequalization" => {
                Some(Box::new(tools::image_analysis::HistogramEqualization::new()))
            }
            "histogrammatching" => Some(Box::new(tools::image_analysis::HistogramMatching::new())),
            "histogrammatchingtwoimages" => Some(Box::new(
                tools::image_analysis::HistogramMatchingTwoImages::new(),
            )),
            "ihstorgb" => Some(Box::new(tools::image_analysis::IhsToRgb::new())),
            "imagestackprofile" => Some(Box::new(tools::image_analysis::ImageStackProfile::new())),
            "integralimage" => Some(Box::new(tools::image_analysis::IntegralImage::new())),
            "kmeansclustering" => Some(Box::new(tools::image_analysis::KMeansClustering::new())),
            "knearestmeanfilter" => {
                Some(Box::new(tools::image_analysis::KNearestMeanFilter::new()))
            }
            "laplacianfilter" => Some(Box::new(tools::image_analysis::LaplacianFilter::new())),
            "laplacianofgaussianfilter" => Some(Box::new(
                tools::image_analysis::LaplacianOfGaussianFilter::new(),
            )),
            "leefilter" => Some(Box::new(tools::image_analysis::LeeFilter::new())),
            "linedetectionfilter" => {
                Some(Box::new(tools::image_analysis::LineDetectionFilter::new()))
            }
            "linethinning" => Some(Box::new(tools::image_analysis::LineThinning::new())),
            "majorityfilter" => Some(Box::new(tools::image_analysis::MajorityFilter::new())),
            "maximumfilter" => Some(Box::new(tools::image_analysis::MaximumFilter::new())),
            "minmaxcontraststretch" => {
                Some(Box::new(tools::image_analysis::MinMaxContrastStretch::new()))
            }
            "meanfilter" => Some(Box::new(tools::image_analysis::MeanFilter::new())),
            "medianfilter" => Some(Box::new(tools::image_analysis::MedianFilter::new())),
            "minimumfilter" => Some(Box::new(tools::image_analysis::MinimumFilter::new())),
            "modifiedkmeansclustering" => Some(Box::new(
                tools::image_analysis::ModifiedKMeansClustering::new(),
            )),
            "mosaic" => Some(Box::new(tools::image_analysis::Mosaic::new())),
            "normalizeddifferencevegetationindex" => Some(Box::new(
                tools::image_analysis::NormalizedDifferenceVegetationIndex::new(),
            )),
            "olympicfilter" => Some(Box::new(tools::image_analysis::OlympicFilter::new())),
            "opening" => Some(Box::new(tools::image_analysis::Opening::new())),
            "panchromaticsharpening" => Some(Box::new(
                tools::image_analysis::PanchromaticSharpening::new(),
            )),
            "percentagecontraststretch" => Some(Box::new(
                tools::image_analysis::PercentageContrastStretch::new(),
            )),
            "percentilefilter" => Some(Box::new(tools::image_analysis::PercentileFilter::new())),
            "prewittfilter" => Some(Box::new(tools::image_analysis::PrewittFilter::new())),
            "rangefilter" => Some(Box::new(tools::image_analysis::RangeFilter::new())),
            "removespurs" => Some(Box::new(tools::image_analysis::RemoveSpurs::new())),
            "resample" => Some(Box::new(tools::image_analysis::Resample::new())),
            "rgbtoihs" => Some(Box::new(tools::image_analysis::RgbToIhs::new())),
            "robertscrossfilter" => {
                Some(Box::new(tools::image_analysis::RobertsCrossFilter::new()))
            }
            "scharrfilter" => Some(Box::new(tools::image_analysis::ScharrFilter::new())),
            "sigmoidalcontraststretch" => Some(Box::new(
                tools::image_analysis::SigmoidalContrastStretch::new(),
            )),
            "sobelfilter" => Some(Box::new(tools::image_analysis::SobelFilter::new())),
            "splitcolourcomposite" => {
                Some(Box::new(tools::image_analysis::SplitColourComposite::new()))
            }
            "standarddeviationcontraststretch" => Some(Box::new(
                tools::image_analysis::StandardDeviationContrastStretch::new(),
            )),
            "standarddeviationfilter" => Some(Box::new(
                tools::image_analysis::StandardDeviationFilter::new(),
            )),
            "thickenrasterline" => Some(Box::new(tools::image_analysis::ThickenRasterLine::new())),
            "tophattransform" => Some(Box::new(tools::image_analysis::TophatTransform::new())),
            "totalfilter" => Some(Box::new(tools::image_analysis::TotalFilter::new())),
            "unsharpmasking" => Some(Box::new(tools::image_analysis::UnsharpMasking::new())),
            "userdefinedweightsfilter" => Some(Box::new(
                tools::image_analysis::UserDefinedWeightsFilter::new(),
            )),
            "writefunctionmemoryinsertion" => Some(Box::new(
                tools::image_analysis::WriteFunctionMemoryInsertion::new(),
            )),

            // lidar_analysis
            "blockmaximum" => Some(Box::new(tools::lidar_analysis::BlockMaximum::new())),
            "blockminimum" => Some(Box::new(tools::lidar_analysis::BlockMinimum::new())),
            "classifyoverlappoints" => {
                Some(Box::new(tools::lidar_analysis::ClassifyOverlapPoints::new()))
            }
            "cliplidartopolygon" => {
                Some(Box::new(tools::lidar_analysis::ClipLidarToPolygon::new()))
            }
            "erasepolygonfromlidar" => {
                Some(Box::new(tools::lidar_analysis::ErasePolygonFromLidar::new()))
            }
            "filterlidarscanangles" => {
                Some(Box::new(tools::lidar_analysis::FilterLidarScanAngles::new()))
            }
            "findflightlineedgepoints" => Some(Box::new(
                tools::lidar_analysis::FindFlightlineEdgePoints::new(),
            )),
            "flightlineoverlap" => Some(Box::new(tools::lidar_analysis::FlightlineOverlap::new())),
            "lastoascii" => Some(Box::new(tools::lidar_analysis::LasToAscii::new())),
            "lastomultipointshapefile" => Some(Box::new(
                tools::lidar_analysis::LasToMultipointShapefile::new(),
            )),
            "lidarcolourize" => Some(Box::new(tools::lidar_analysis::LidarColourize::new())),
            "lidarelevationslice" => {
                Some(Box::new(tools::lidar_analysis::LidarElevationSlice::new()))
            }
            "lidargroundpointfilter" => Some(Box::new(
                tools::lidar_analysis::LidarGroundPointFilter::new(),
            )),
            "lidarhillshade" => Some(Box::new(tools::lidar_analysis::LidarHillshade::new())),
            "lidarhistogram" => Some(Box::new(tools::lidar_analysis::LidarHistogram::new())),
            "lidaridwinterpolation" => {
                Some(Box::new(tools::lidar_analysis::LidarIdwInterpolation::new()))
            }
            "lidarinfo" => Some(Box::new(tools::lidar_analysis::LidarInfo::new())),
            "lidarjoin" => Some(Box::new(tools::lidar_analysis::LidarJoin::new())),
            "lidarkappaindex" => Some(Box::new(tools::lidar_analysis::LidarKappaIndex::new())),
            "lidarnearestneighbourgridding" => Some(Box::new(
                tools::lidar_analysis::LidarNearestNeighbourGridding::new(),
            )),
            "lidarpointdensity" => Some(Box::new(tools::lidar_analysis::LidarPointDensity::new())),
            "lidarpointstats" => Some(Box::new(tools::lidar_analysis::LidarPointStats::new())),
            "lidarremoveduplicates" => {
                Some(Box::new(tools::lidar_analysis::LidarRemoveDuplicates::new()))
            }
            "lidarremoveoutliers" => {
                Some(Box::new(tools::lidar_analysis::LidarRemoveOutliers::new()))
            }
            "lidarsegmentation" => Some(Box::new(tools::lidar_analysis::LidarSegmentation::new())),
            "lidarsegmentationbasedfilter" => Some(Box::new(
                tools::lidar_analysis::LidarSegmentationBasedFilter::new(),
            )),
            "lidarthin" => Some(Box::new(tools::lidar_analysis::LidarThin::new())),
            "lidarthinhighdensity" => {
                Some(Box::new(tools::lidar_analysis::LidarThinHighDensity::new()))
            }
            "lidartile" => Some(Box::new(tools::lidar_analysis::LidarTile::new())),
            "lidartilefootprint" => {
                Some(Box::new(tools::lidar_analysis::LidarTileFootprint::new()))
            }
            "lidartophattransform" => {
                Some(Box::new(tools::lidar_analysis::LidarTophatTransform::new()))
            }
            "normalvectors" => Some(Box::new(tools::lidar_analysis::NormalVectors::new())),
            "selecttilesbypolygon" => {
                Some(Box::new(tools::lidar_analysis::SelectTilesByPolygon::new()))
            }

            // mathematical and statistical_analysis
            "absolutevalue" => Some(Box::new(tools::math_stat_analysis::AbsoluteValue::new())),
            "add" => Some(Box::new(tools::math_stat_analysis::Add::new())),
            "and" => Some(Box::new(tools::math_stat_analysis::And::new())),
            "anova" => Some(Box::new(tools::math_stat_analysis::Anova::new())),
            "arccos" => Some(Box::new(tools::math_stat_analysis::ArcCos::new())),
            "arcsin" => Some(Box::new(tools::math_stat_analysis::ArcSin::new())),
            "arctan" => Some(Box::new(tools::math_stat_analysis::ArcTan::new())),
            "atan2" => Some(Box::new(tools::math_stat_analysis::Atan2::new())),
            "attributecorrelation" => Some(Box::new(
                tools::math_stat_analysis::AttributeCorrelation::new(),
            )),
            "attributehistogram" => Some(Box::new(
                tools::math_stat_analysis::AttributeHistogram::new(),
            )),
            "attributescattergram" => Some(Box::new(
                tools::math_stat_analysis::AttributeScattergram::new(),
            )),
            "ceil" => Some(Box::new(tools::math_stat_analysis::Ceil::new())),
            "cos" => Some(Box::new(tools::math_stat_analysis::Cos::new())),
            "cosh" => Some(Box::new(tools::math_stat_analysis::Cosh::new())),
            "crispnessindex" => Some(Box::new(tools::math_stat_analysis::CrispnessIndex::new())),
            "crosstabulation" => Some(Box::new(tools::math_stat_analysis::CrossTabulation::new())),
            "cumulativedistribution" => Some(Box::new(
                tools::math_stat_analysis::CumulativeDistribution::new(),
            )),
            "decrement" => Some(Box::new(tools::math_stat_analysis::Decrement::new())),
            "divide" => Some(Box::new(tools::math_stat_analysis::Divide::new())),
            "equalto" => Some(Box::new(tools::math_stat_analysis::EqualTo::new())),
            "exp" => Some(Box::new(tools::math_stat_analysis::Exp::new())),
            "exp2" => Some(Box::new(tools::math_stat_analysis::Exp2::new())),
            "extractrasterstatistics" => Some(Box::new(
                tools::math_stat_analysis::ExtractRasterStatistics::new(),
            )),
            "floor" => Some(Box::new(tools::math_stat_analysis::Floor::new())),
            "greaterthan" => Some(Box::new(tools::math_stat_analysis::GreaterThan::new())),
            "imageautocorrelation" => Some(Box::new(
                tools::math_stat_analysis::ImageAutocorrelation::new(),
            )),
            "imagecorrelation" => {
                Some(Box::new(tools::math_stat_analysis::ImageCorrelation::new()))
            }
            "imageregression" => Some(Box::new(tools::math_stat_analysis::ImageRegression::new())),
            "increment" => Some(Box::new(tools::math_stat_analysis::Increment::new())),
            "inplaceadd" => Some(Box::new(tools::math_stat_analysis::InPlaceAdd::new())),
            "inplacedivide" => Some(Box::new(tools::math_stat_analysis::InPlaceDivide::new())),
            "inplacemultiply" => Some(Box::new(tools::math_stat_analysis::InPlaceMultiply::new())),
            "inplacesubtract" => Some(Box::new(tools::math_stat_analysis::InPlaceSubtract::new())),
            "integerdivision" => Some(Box::new(tools::math_stat_analysis::IntegerDivision::new())),
            "isnodata" => Some(Box::new(tools::math_stat_analysis::IsNoData::new())),
            "kappaindex" => Some(Box::new(tools::math_stat_analysis::KappaIndex::new())),
            "kstestfornormality" => Some(Box::new(
                tools::math_stat_analysis::KSTestForNormality::new(),
            )),
            "lessthan" => Some(Box::new(tools::math_stat_analysis::LessThan::new())),
            "listuniquevalues" => {
                Some(Box::new(tools::math_stat_analysis::ListUniqueValues::new()))
            }
            "log10" => Some(Box::new(tools::math_stat_analysis::Log10::new())),
            "log2" => Some(Box::new(tools::math_stat_analysis::Log2::new())),
            "ln" => Some(Box::new(tools::math_stat_analysis::Ln::new())),
            "max" => Some(Box::new(tools::math_stat_analysis::Max::new())),
            "min" => Some(Box::new(tools::math_stat_analysis::Min::new())),
            "modulo" => Some(Box::new(tools::math_stat_analysis::Modulo::new())),
            "multiply" => Some(Box::new(tools::math_stat_analysis::Multiply::new())),
            "negate" => Some(Box::new(tools::math_stat_analysis::Negate::new())),
            "not" => Some(Box::new(tools::math_stat_analysis::Not::new())),
            "notequalto" => Some(Box::new(tools::math_stat_analysis::NotEqualTo::new())),
            "or" => Some(Box::new(tools::math_stat_analysis::Or::new())),
            "power" => Some(Box::new(tools::math_stat_analysis::Power::new())),
            "principalcomponentanalysis" => Some(Box::new(
                tools::math_stat_analysis::PrincipalComponentAnalysis::new(),
            )),
            "quantiles" => Some(Box::new(tools::math_stat_analysis::Quantiles::new())),
            "randomfield" => Some(Box::new(tools::math_stat_analysis::RandomField::new())),
            "randomsample" => Some(Box::new(tools::math_stat_analysis::RandomSample::new())),
            "rasterhistogram" => Some(Box::new(tools::math_stat_analysis::RasterHistogram::new())),
            "rastersummarystats" => Some(Box::new(
                tools::math_stat_analysis::RasterSummaryStats::new(),
            )),
            "reciprocal" => Some(Box::new(tools::math_stat_analysis::Reciprocal::new())),
            "rescalevaluerange" => {
                Some(Box::new(tools::math_stat_analysis::RescaleValueRange::new()))
            }
            "rootmeansquareerror" => Some(Box::new(
                tools::math_stat_analysis::RootMeanSquareError::new(),
            )),
            "round" => Some(Box::new(tools::math_stat_analysis::Round::new())),
            "sin" => Some(Box::new(tools::math_stat_analysis::Sin::new())),
            "sinh" => Some(Box::new(tools::math_stat_analysis::Sinh::new())),
            "square" => Some(Box::new(tools::math_stat_analysis::Square::new())),
            "squareroot" => Some(Box::new(tools::math_stat_analysis::SquareRoot::new())),
            "subtract" => Some(Box::new(tools::math_stat_analysis::Subtract::new())),
            "tan" => Some(Box::new(tools::math_stat_analysis::Tan::new())),
            "tanh" => Some(Box::new(tools::math_stat_analysis::Tanh::new())),
            "todegrees" => Some(Box::new(tools::math_stat_analysis::ToDegrees::new())),
            "toradians" => Some(Box::new(tools::math_stat_analysis::ToRadians::new())),
            "trendsurface" => Some(Box::new(tools::math_stat_analysis::TrendSurface::new())),
            "trendsurfacevectorpoints" => Some(Box::new(
                tools::math_stat_analysis::TrendSurfaceVectorPoints::new(),
            )),
            "truncate" => Some(Box::new(tools::math_stat_analysis::Truncate::new())),
            "turningbandssimulation" => Some(Box::new(
                tools::math_stat_analysis::TurningBandsSimulation::new(),
            )),
            "xor" => Some(Box::new(tools::math_stat_analysis::Xor::new())),
            "zscores" => Some(Box::new(tools::math_stat_analysis::ZScores::new())),

            // stream_network_analysis
            "distancetooutlet" => Some(Box::new(
                tools::stream_network_analysis::DistanceToOutlet::new(),
            )),
            "extractstreams" => Some(Box::new(
                tools::stream_network_analysis::ExtractStreams::new(),
            )),
            "extractvalleys" => Some(Box::new(
                tools::stream_network_analysis::ExtractValleys::new(),
            )),
            "farthestchannelhead" => Some(Box::new(
                tools::stream_network_analysis::FarthestChannelHead::new(),
            )),
            "findmainstem" => Some(Box::new(tools::stream_network_analysis::FindMainStem::new())),
            "hackstreamorder" => Some(Box::new(
                tools::stream_network_analysis::HackStreamOrder::new(),
            )),
            "hortonstreamorder" => Some(Box::new(
                tools::stream_network_analysis::HortonStreamOrder::new(),
            )),
            "lengthofupstreamchannels" => Some(Box::new(
                tools::stream_network_analysis::LengthOfUpstreamChannels::new(),
            )),
            "longprofile" => Some(Box::new(tools::stream_network_analysis::LongProfile::new())),
            "longprofilefrompoints" => Some(Box::new(
                tools::stream_network_analysis::LongProfileFromPoints::new(),
            )),
            "rasterizestreams" => Some(Box::new(
                tools::stream_network_analysis::RasterizeStreams::new(),
            )),
            "removeshortstreams" => Some(Box::new(
                tools::stream_network_analysis::RemoveShortStreams::new(),
            )),
            "shrevestreammagnitude" => Some(Box::new(
                tools::stream_network_analysis::ShreveStreamMagnitude::new(),
            )),
            "strahlerstreamorder" => Some(Box::new(
                tools::stream_network_analysis::StrahlerStreamOrder::new(),
            )),
            "streamlinkclass" => Some(Box::new(
                tools::stream_network_analysis::StreamLinkClass::new(),
            )),
            "streamlinkidentifier" => Some(Box::new(
                tools::stream_network_analysis::StreamLinkIdentifier::new(),
            )),
            "streamlinklength" => Some(Box::new(
                tools::stream_network_analysis::StreamLinkLength::new(),
            )),
            "streamlinkslope" => Some(Box::new(
                tools::stream_network_analysis::StreamLinkSlope::new(),
            )),
            "streamslopecontinuous" => Some(Box::new(
                tools::stream_network_analysis::StreamSlopeContinuous::new(),
            )),
            "topologicalstreamorder" => Some(Box::new(
                tools::stream_network_analysis::TopologicalStreamOrder::new(),
            )),
            "tributaryidentifier" => Some(Box::new(
                tools::stream_network_analysis::TributaryIdentifier::new(),
            )),

            // terrain_analysis
            "aspect" => Some(Box::new(tools::terrain_analysis::Aspect::new())),
            "devfrommeanelev" => Some(Box::new(tools::terrain_analysis::DevFromMeanElev::new())),
            "difffrommeanelev" => Some(Box::new(tools::terrain_analysis::DiffFromMeanElev::new())),
            "directionalrelief" => {
                Some(Box::new(tools::terrain_analysis::DirectionalRelief::new()))
            }
            "downslopeindex" => Some(Box::new(tools::terrain_analysis::DownslopeIndex::new())),
            "elevabovepit" => Some(Box::new(tools::terrain_analysis::ElevAbovePit::new())),
            "elevpercentile" => Some(Box::new(tools::terrain_analysis::ElevPercentile::new())),
            "elevrelativetominmax" => Some(Box::new(
                tools::terrain_analysis::ElevRelativeToMinMax::new(),
            )),
            "elevrelativetowatershedminmax" => Some(Box::new(
                tools::terrain_analysis::ElevRelativeToWatershedMinMax::new(),
            )),
            "featurepreservingdenoise" => Some(Box::new(
                tools::terrain_analysis::FeaturePreservingDenoise::new(),
            )),
            "featurepreservingdenoisewithexclusions" => Some(Box::new(
                tools::terrain_analysis::FeaturePreservingDenoiseWithExclusions::new(),
            )),
            "fetchanalysis" => Some(Box::new(tools::terrain_analysis::FetchAnalysis::new())),
            "fillmissingdata" => Some(Box::new(tools::terrain_analysis::FillMissingData::new())),
            "findridges" => Some(Box::new(tools::terrain_analysis::FindRidges::new())),
            "hillshade" => Some(Box::new(tools::terrain_analysis::Hillshade::new())),
            "horizonangle" => Some(Box::new(tools::terrain_analysis::HorizonAngle::new())),
            "hypsometricanalysis" => {
                Some(Box::new(tools::terrain_analysis::HypsometricAnalysis::new()))
            }
            "maxanisotropydev" => Some(Box::new(tools::terrain_analysis::MaxAnisotropyDev::new())),
            "maxanisotropydevsignature" => Some(Box::new(
                tools::terrain_analysis::MaxAnisotropyDevSignature::new(),
            )),
            "maxbranchlength" => Some(Box::new(tools::terrain_analysis::MaxBranchLength::new())),
            "maxdifferencefrommean" => Some(Box::new(
                tools::terrain_analysis::MaxDifferenceFromMean::new(),
            )),
            "maxdownslopeelevchange" => Some(Box::new(
                tools::terrain_analysis::MaxDownslopeElevChange::new(),
            )),
            "maxelevationdeviation" => Some(Box::new(
                tools::terrain_analysis::MaxElevationDeviation::new(),
            )),
            "maxelevdevsignature" => {
                Some(Box::new(tools::terrain_analysis::MaxElevDevSignature::new()))
            }
            "mindownslopeelevchange" => Some(Box::new(
                tools::terrain_analysis::MinDownslopeElevChange::new(),
            )),
            "multiscaleroughness" => {
                Some(Box::new(tools::terrain_analysis::MultiscaleRoughness::new()))
            }
            "multiscaleroughnesssignature" => Some(Box::new(
                tools::terrain_analysis::MultiscaleRoughnessSignature::new(),
            )),
            "multiscaletopographicpositionimage" => Some(Box::new(
                tools::terrain_analysis::MultiscaleTopographicPositionImage::new(),
            )),
            "numdownslopeneighbours" => Some(Box::new(
                tools::terrain_analysis::NumDownslopeNeighbours::new(),
            )),
            "numupslopeneighbours" => Some(Box::new(
                tools::terrain_analysis::NumUpslopeNeighbours::new(),
            )),
            "pennocklandformclass" => Some(Box::new(
                tools::terrain_analysis::PennockLandformClass::new(),
            )),
            "percentelevrange" => Some(Box::new(tools::terrain_analysis::PercentElevRange::new())),
            "plancurvature" => Some(Box::new(tools::terrain_analysis::PlanCurvature::new())),
            "profilecurvature" => Some(Box::new(tools::terrain_analysis::ProfileCurvature::new())),
            "profile" => Some(Box::new(tools::terrain_analysis::Profile::new())),
            "relativeaspect" => Some(Box::new(tools::terrain_analysis::RelativeAspect::new())),
            "relativestreampowerindex" => Some(Box::new(
                tools::terrain_analysis::RelativeStreamPowerIndex::new(),
            )),
            "relativetopographicposition" => Some(Box::new(
                tools::terrain_analysis::RelativeTopographicPosition::new(),
            )),
            "removeoffterrainobjects" => Some(Box::new(
                tools::terrain_analysis::RemoveOffTerrainObjects::new(),
            )),
            "ruggednessindex" => Some(Box::new(tools::terrain_analysis::RuggednessIndex::new())),
            "sedimenttransportindex" => Some(Box::new(
                tools::terrain_analysis::SedimentTransportIndex::new(),
            )),
            "slope" => Some(Box::new(tools::terrain_analysis::Slope::new())),
            "slopevselevationplot" => Some(Box::new(
                tools::terrain_analysis::SlopeVsElevationPlot::new(),
            )),
            "standarddeviationofslope" => Some(Box::new(
                tools::terrain_analysis::StandardDeviationOfSlope::new(),
            )),
            "tangentialcurvature" => {
                Some(Box::new(tools::terrain_analysis::TangentialCurvature::new()))
            }
            "totalcurvature" => Some(Box::new(tools::terrain_analysis::TotalCurvature::new())),
            "viewshed" => Some(Box::new(tools::terrain_analysis::Viewshed::new())),
            "visibilityindex" => Some(Box::new(tools::terrain_analysis::VisibilityIndex::new())),
            "wetnessindex" => Some(Box::new(tools::terrain_analysis::WetnessIndex::new())),

            _ => None,
        }
    }

    pub fn run_tool(&self, tool_name: String, args: Vec<String>) -> Result<(), Error> {
        // if !working_dir.is_empty() {
        //     tool_args_vec.insert(0, format!("--wd={}", working_dir));
        // }

        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => return tool.run(args, &self.working_dir, self.verbose),
            None => {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Unrecognized tool name {}.", tool_name),
                ))
            }
        }
    }

    pub fn tool_help(&self, tool_name: String) -> Result<(), Error> {
        if !tool_name.is_empty() {
            match self.get_tool(tool_name.as_ref()) {
                Some(tool) => println!("{}", get_help(tool)),
                None => {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        format!("Unrecognized tool name {}.", tool_name),
                    ))
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

    pub fn tool_parameters(&self, tool_name: String) -> Result<(), Error> {
        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => println!("{}", tool.get_tool_parameters()),
            None => {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Unrecognized tool name {}.", tool_name),
                ))
            }
        }
        Ok(())
    }

    pub fn toolbox(&self, tool_name: String) -> Result<(), Error> {
        if !tool_name.is_empty() {
            match self.get_tool(tool_name.as_ref()) {
                Some(tool) => println!("{}", tool.get_toolbox()),
                None => {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        format!("Unrecognized tool name {}.", tool_name),
                    ))
                }
            }
        } else {
            for val in &self.tool_names {
                let tool = self.get_tool(&val).unwrap();
                let toolbox = tool.get_toolbox();
                println!("{}: {}\n", val, toolbox);
            }
        }
        Ok(())
    }

    pub fn list_tools(&self) {
        let mut tool_details: Vec<(String, String)> = Vec::new();

        for val in &self.tool_names {
            let tool = self.get_tool(&val).unwrap();
            tool_details.push(get_name_and_description(tool));
        }

        let mut ret = format!("All {} Available Tools:\n", tool_details.len());
        for i in 0..tool_details.len() {
            ret.push_str(&format!("{}: {}\n\n", tool_details[i].0, tool_details[i].1));
        }
        println!("{}", ret);
    }

    pub fn list_tools_with_keywords(&self, keywords: Vec<String>) {
        let mut tool_details: Vec<(String, String)> = Vec::new();
        for val in &self.tool_names {
            let tool = self.get_tool(&val).unwrap();
            let toolbox = tool.get_toolbox();
            let (nm, des) = get_name_and_description(tool);
            for kw in &keywords {
                if nm.to_lowercase().contains(&(kw.to_lowercase()))
                    || des.to_lowercase().contains(&(kw.to_lowercase()))
                    || toolbox.to_lowercase().contains(&(kw.to_lowercase()))
                {
                    tool_details.push(get_name_and_description(self.get_tool(&val).unwrap()));
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
        let repo = String::from("https://github.com/jblindsay/whitebox-tools//tree/master/");
        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => println!("{}{}", repo, tool.get_source_file()),
            None => {
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

fn get_help<'a>(wt: Box<WhiteboxTool + 'a>) -> String {
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

fn get_name_and_description<'a>(wt: Box<WhiteboxTool + 'a>) -> (String, String) {
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
