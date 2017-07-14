pub mod data_tools;
pub mod gis_analysis;
pub mod hydro_analysis;
pub mod image_analysis;
pub mod lidar_analysis;
pub mod math_stat_analysis;
pub mod stream_network_analysis;
pub mod terrain_analysis;

use tools;
use std::io::{Error, ErrorKind};

#[derive(Default)]
pub struct ToolManager {
    pub working_dir: String,
    pub verbose: bool,
    tool_names: Vec<String>,
}

impl ToolManager {
    pub fn new<'a>(working_directory: &'a str,
                   verbose_mode: &'a bool)
                   -> Result<ToolManager, Error> {
        let mut tool_names = vec![];
        // data_tools
        tool_names.push("ConvertNodataToZero".to_string());
        tool_names.push("ConvertRasterFormat".to_string());
        tool_names.push("NewRasterFromBase".to_string());

        // gis_analysis
        tool_names.push("AverageOverlay".to_string());
        tool_names.push("BufferRaster".to_string());
        tool_names.push("Clump".to_string());
        tool_names.push("CostAllocation".to_string());
        tool_names.push("CostDistance".to_string());
        tool_names.push("CostPathway".to_string());
        tool_names.push("CreatePlane".to_string());
        tool_names.push("EdgeProportion".to_string());
        tool_names.push("EuclideanAllocation".to_string());
        tool_names.push("EuclideanDistance".to_string());
        tool_names.push("FindPatchOrClassEdgeCells".to_string());
        tool_names.push("HighestPosition".to_string());
        tool_names.push("LowestPosition".to_string());
        tool_names.push("MaxAbsoluteOverlay".to_string());
        tool_names.push("MaxOverlay".to_string());
        tool_names.push("MinAbsoluteOverlay".to_string());
        tool_names.push("MinOverlay".to_string());
        tool_names.push("PercentEqualTo".to_string());
        tool_names.push("PercentGreaterThan".to_string());
        tool_names.push("PercentLessThan".to_string());
        tool_names.push("PickFromList".to_string());
        tool_names.push("ReclassEqualInterval".to_string());
        tool_names.push("WeightedSum".to_string());

        // hydro_analysis
        tool_names.push("AverageUpslopeFlowpathLength".to_string());
        tool_names.push("Basins".to_string());
        tool_names.push("BreachDepressions".to_string());
        tool_names.push("D8FlowAccumulation".to_string());
        tool_names.push("D8Pointer".to_string());
        tool_names.push("DepthInSink".to_string());
        tool_names.push("DInfFlowAccumulation".to_string());
        tool_names.push("DInfPointer".to_string());
        tool_names.push("DownslopeDistanceToStream".to_string());
        tool_names.push("DownslopeFlowpathLength".to_string());
        tool_names.push("ElevationAboveStream".to_string());
        tool_names.push("FD8FlowAccumulation".to_string());
        tool_names.push("FD8Pointer".to_string());
        tool_names.push("FillDepressions".to_string());
        tool_names.push("FillSingleCellPits".to_string());
        tool_names.push("FindNoFlowCells".to_string());
        tool_names.push("FindParallelFlow".to_string());
        tool_names.push("FloodOrder".to_string());
        tool_names.push("FlowLengthDiff".to_string());
        tool_names.push("JensonSnapPourPoints".to_string());
        tool_names.push("MaxUpslopeFlowpathLength".to_string());
        tool_names.push("NumInflowingNeighbours".to_string());
        tool_names.push("Sink".to_string());
        tool_names.push("SnapPourPoints".to_string());
        tool_names.push("StrahlerOrderBasins".to_string());
        tool_names.push("Subbasins".to_string());
        tool_names.push("TraceDownslopeFlowpaths".to_string());
        tool_names.push("Watershed".to_string());

        // image_analysis
        tool_names.push("AdaptiveFilter".to_string());
        tool_names.push("BilateralFilter".to_string());
        tool_names.push("Closing".to_string());
        tool_names.push("ConservativeSmoothingFilter".to_string());
        tool_names.push("DiversityFilter".to_string());
        tool_names.push("DiffOfGaussianFilter".to_string());
        tool_names.push("EmbossFilter".to_string());
        tool_names.push("FlipImage".to_string());
        tool_names.push("GammaCorrection".to_string());
        tool_names.push("GaussianFilter".to_string());
        tool_names.push("HighPassFilter".to_string());
        tool_names.push("IntegralImage".to_string());
        tool_names.push("LaplacianFilter".to_string());
        tool_names.push("LaplacianOfGaussianFilter".to_string());
        tool_names.push("LineDetectionFilter".to_string());
        tool_names.push("LineThinning".to_string());
        tool_names.push("MajorityFilter".to_string());
        tool_names.push("MaximumFilter".to_string());
        tool_names.push("MeanFilter".to_string());
        tool_names.push("MinMaxContrastStretch".to_string());
        tool_names.push("MinimumFilter".to_string());
        tool_names.push("NormalizedDifferenceVegetationIndex".to_string());
        tool_names.push("OlympicFilter".to_string());
        tool_names.push("Opening".to_string());
        tool_names.push("PercentageContrastStretch".to_string());
        tool_names.push("PercentileFilter".to_string());
        tool_names.push("PrewittFilter".to_string());
        tool_names.push("RobertsCrossFilter".to_string());
        tool_names.push("RangeFilter".to_string());
        tool_names.push("RemoveSpurs".to_string());
        tool_names.push("ScharrFilter".to_string());
        tool_names.push("SigmoidalContrastStretch".to_string());
        tool_names.push("SobelFilter".to_string());
        tool_names.push("StandardDeviationContrastStretch".to_string());
        tool_names.push("StandardDeviationFilter".to_string());
        tool_names.push("ThickenRasterLine".to_string());
        tool_names.push("TophatTransform".to_string());
        tool_names.push("TotalFilter".to_string());

        // lidar_analysis
        tool_names.push("BlockMaximum".to_string());
        tool_names.push("BlockMinimum".to_string());
        tool_names.push("FlightlineOverlap".to_string());
        tool_names.push("LidarElevationSlice".to_string());
        tool_names.push("LidarGroundPointFilter".to_string());
        tool_names.push("LidarHillshade".to_string());
        tool_names.push("LidarIdwInterpolation".to_string());
        tool_names.push("LidarInfo".to_string());
        tool_names.push("LidarJoin".to_string());
        tool_names.push("LidarNearestNeighbourGridding".to_string());
        tool_names.push("LidarPointDensity".to_string());
        tool_names.push("LidarTile".to_string());
        tool_names.push("LidarTophatTransform".to_string());
        tool_names.push("NormalVectors".to_string());

        // mathematical and statistical_analysis
        tool_names.push("AbsoluteValue".to_string());
        tool_names.push("ArcCos".to_string());
        tool_names.push("ArcSin".to_string());
        tool_names.push("ArcTan".to_string());
        tool_names.push("Atan2".to_string());
        tool_names.push("Add".to_string());
        tool_names.push("And".to_string());
        tool_names.push("Ceil".to_string());
        tool_names.push("Cos".to_string());
        tool_names.push("Cosh".to_string());
        tool_names.push("Decrement".to_string());
        tool_names.push("Divide".to_string());
        tool_names.push("EqualTo".to_string());
        tool_names.push("Exp".to_string());
        tool_names.push("Exp2".to_string());
        tool_names.push("Floor".to_string());
        tool_names.push("GreaterThan".to_string());
        tool_names.push("Increment".to_string());
        tool_names.push("IntegerDivision".to_string());
        tool_names.push("IsNoData".to_string());
        tool_names.push("LessThan".to_string());
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
        tool_names.push("Quantiles".to_string());
        tool_names.push("RandomField".to_string());
        tool_names.push("RasterSummaryStats".to_string());
        tool_names.push("Reciprocal".to_string());
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
        tool_names.push("ElevAbovePit".to_string());
        tool_names.push("ElevPercentile".to_string());
        tool_names.push("ElevRelativeToMinMax".to_string());
        tool_names.push("ElevRelativeToWatershedMinMax".to_string());
        tool_names.push("FetchAnalysis".to_string());
        tool_names.push("FillMissingData".to_string());
        tool_names.push("Hillshade".to_string());
        tool_names.push("HorizonAngle".to_string());
        tool_names.push("MaxBranchLength".to_string());
        tool_names.push("MaxDownslopeElevChange".to_string());
        tool_names.push("MinDownslopeElevChange".to_string());
        tool_names.push("NumDownslopeNeighbours".to_string());
        tool_names.push("NumUpslopeNeighbours".to_string());
        tool_names.push("PennockLandformClass".to_string());
        tool_names.push("PercentElevRange".to_string());
        tool_names.push("PlanCurvature".to_string());
        tool_names.push("ProfileCurvature".to_string());
        tool_names.push("RelativeAspect".to_string());
        tool_names.push("RelativeStreamPowerIndex".to_string());
        tool_names.push("RelativeTopographicPosition".to_string());
        tool_names.push("RemoveOffTerrainObjects".to_string());
        tool_names.push("RuggednessIndex".to_string());
        tool_names.push("SedimentTransportIndex".to_string());
        tool_names.push("Slope".to_string());
        tool_names.push("TangentialCurvature".to_string());
        tool_names.push("TotalCurvature".to_string());
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
            "newrasterfrombase" => Some(Box::new(tools::data_tools::NewRasterFromBase::new())),

            // gis_analysis
            "averageoverlay" => Some(Box::new(tools::gis_analysis::AverageOverlay::new())),
            "bufferraster" => Some(Box::new(tools::gis_analysis::BufferRaster::new())),
            "clump" => Some(Box::new(tools::gis_analysis::Clump::new())),
            "costallocation" => Some(Box::new(tools::gis_analysis::CostAllocation::new())),
            "costdistance" => Some(Box::new(tools::gis_analysis::CostDistance::new())),
            "costpathway" => Some(Box::new(tools::gis_analysis::CostPathway::new())),
            "createplane" => Some(Box::new(tools::gis_analysis::CreatePlane::new())),
            "edgeproportion" => Some(Box::new(tools::gis_analysis::EdgeProportion::new())),
            "euclideanallocation" => {
                Some(Box::new(tools::gis_analysis::EuclideanAllocation::new()))
            }
            "euclideandistance" => Some(Box::new(tools::gis_analysis::EuclideanDistance::new())),
            "findpatchorclassedgecells" => Some(Box::new(tools::gis_analysis::FindPatchOrClassEdgeCells::new())),
            "highestposition" => Some(Box::new(tools::gis_analysis::HighestPosition::new())),
            "lowestposition" => Some(Box::new(tools::gis_analysis::LowestPosition::new())),
            "maxabsoluteoverlay" => Some(Box::new(tools::gis_analysis::MaxAbsoluteOverlay::new())),
            "maxoverlay" => Some(Box::new(tools::gis_analysis::MaxOverlay::new())),
            "minabsoluteoverlay" => Some(Box::new(tools::gis_analysis::MinAbsoluteOverlay::new())),
            "minoverlay" => Some(Box::new(tools::gis_analysis::MinOverlay::new())),
            "percentequalto" => Some(Box::new(tools::gis_analysis::PercentEqualTo::new())),
            "percentgreaterthan" => Some(Box::new(tools::gis_analysis::PercentGreaterThan::new())),
            "percentlessthan" => Some(Box::new(tools::gis_analysis::PercentLessThan::new())),
            "pickfromlist" => Some(Box::new(tools::gis_analysis::PickFromList::new())),
            "reclassequalinterval" => {
                Some(Box::new(tools::gis_analysis::ReclassEqualInterval::new()))
            }
            "weightedsum" => Some(Box::new(tools::gis_analysis::WeightedSum::new())),


            // hydro_analysis
            "averageupslopeflowpathlength" => {
                Some(Box::new(tools::hydro_analysis::AverageUpslopeFlowpathLength::new()))
            }
            "basins" => Some(Box::new(tools::hydro_analysis::Basins::new())),
            "breachdepressions" => Some(Box::new(tools::hydro_analysis::BreachDepressions::new())),
            "d8flowaccumulation" => {
                Some(Box::new(tools::hydro_analysis::D8FlowAccumulation::new()))
            }
            "d8pointer" => Some(Box::new(tools::hydro_analysis::D8Pointer::new())),
            "depthinsink" => Some(Box::new(tools::hydro_analysis::DepthInSink::new())),
            "dinfflowaccumulation" => {
                Some(Box::new(tools::hydro_analysis::DInfFlowAccumulation::new()))
            }
            "dinfpointer" => Some(Box::new(tools::hydro_analysis::DInfPointer::new())),
            "downslopedistancetostream" => {
                Some(Box::new(tools::hydro_analysis::DownslopeDistanceToStream::new()))
            }
            "downslopeflowpathlength" => {
                Some(Box::new(tools::hydro_analysis::DownslopeFlowpathLength::new()))
            }
            "elevationabovestream" => {
                Some(Box::new(tools::hydro_analysis::ElevationAboveStream::new()))
            }
            "fd8flowaccumulation" => {
                Some(Box::new(tools::hydro_analysis::FD8FlowAccumulation::new()))
            }
            "fd8pointer" => Some(Box::new(tools::hydro_analysis::FD8Pointer::new())),
            "filldepressions" => Some(Box::new(tools::hydro_analysis::FillDepressions::new())),
            "fillsinglecellpits" => Some(Box::new(tools::hydro_analysis::FillSingleCellPits::new())),
            "findnoflowcells" => Some(Box::new(tools::hydro_analysis::FindNoFlowCells::new())),
            "findparallelflow" => Some(Box::new(tools::hydro_analysis::FindParallelFlow::new())),
            "floodorder" => Some(Box::new(tools::hydro_analysis::FloodOrder::new())),
            "flowlengthdiff" => Some(Box::new(tools::hydro_analysis::FlowLengthDiff::new())),
            "jensonsnappourpoints" => {
                Some(Box::new(tools::hydro_analysis::JensonSnapPourPoints::new()))
            }
            "maxupslopeflowpathlength" => {
                Some(Box::new(tools::hydro_analysis::MaxUpslopeFlowpathLength::new()))
            }
            "numinflowingneighbours" => {
                Some(Box::new(tools::hydro_analysis::NumInflowingNeighbours::new()))
            }
            "sink" => Some(Box::new(tools::hydro_analysis::Sink::new())),
            "snappourpoints" => Some(Box::new(tools::hydro_analysis::SnapPourPoints::new())),
            "strahlerorderbasins" => Some(Box::new(tools::hydro_analysis::StrahlerOrderBasins::new())),
            "subbasins" => Some(Box::new(tools::hydro_analysis::Subbasins::new())),
            "tracedownslopeflowpaths" => {
                Some(Box::new(tools::hydro_analysis::TraceDownslopeFlowpaths::new()))
            }
            "watershed" => Some(Box::new(tools::hydro_analysis::Watershed::new())),


            // image_analysis
            "adaptivefilter" => Some(Box::new(tools::image_analysis::AdaptiveFilter::new())),
            "bilateralfilter" => Some(Box::new(tools::image_analysis::BilateralFilter::new())),
            "closing" => Some(Box::new(tools::image_analysis::Closing::new())),
            "conservativesmoothingfilter" => {
                Some(Box::new(tools::image_analysis::ConservativeSmoothingFilter::new()))
            }
            "diversityfilter" => Some(Box::new(tools::image_analysis::DiversityFilter::new())),
            "diffofgaussianfilter" => {
                Some(Box::new(tools::image_analysis::DiffOfGaussianFilter::new()))
            }
            "embossfilter" => Some(Box::new(tools::image_analysis::EmbossFilter::new())),
            "flipimage" => Some(Box::new(tools::image_analysis::FlipImage::new())),
            "gammacorrection" => Some(Box::new(tools::image_analysis::GammaCorrection::new())),
            "gaussianfilter" => Some(Box::new(tools::image_analysis::GaussianFilter::new())),
            "highpassfilter" => Some(Box::new(tools::image_analysis::HighPassFilter::new())),
            "integralimage" => Some(Box::new(tools::image_analysis::IntegralImage::new())),
            "laplacianfilter" => Some(Box::new(tools::image_analysis::LaplacianFilter::new())),
            "laplacianofgaussianfilter" => {
                Some(Box::new(tools::image_analysis::LaplacianOfGaussianFilter::new()))
            }
            "linedetectionfilter" => {
                Some(Box::new(tools::image_analysis::LineDetectionFilter::new()))
            }
            "linethinning" => Some(Box::new(tools::image_analysis::LineThinning::new())),
            "majorityfilter" => Some(Box::new(tools::image_analysis::MajorityFilter::new())),
            "maximumfilter" => Some(Box::new(tools::image_analysis::MaximumFilter::new())),
            "minmaxcontraststretch" => Some(Box::new(tools::image_analysis::MinMaxContrastStretch::new())),
            "meanfilter" => Some(Box::new(tools::image_analysis::MeanFilter::new())),
            "minimumfilter" => Some(Box::new(tools::image_analysis::MinimumFilter::new())),
            "normalizeddifferencevegetationindex" => {
                Some(Box::new(tools::image_analysis::NormalizedDifferenceVegetationIndex::new()))
            }
            "olympicfilter" => Some(Box::new(tools::image_analysis::OlympicFilter::new())),
            "opening" => Some(Box::new(tools::image_analysis::Opening::new())),
            "percentagecontraststretch" => Some(Box::new(tools::image_analysis::PercentageContrastStretch::new())),
            "percentilefilter" => Some(Box::new(tools::image_analysis::PercentileFilter::new())),
            "prewittfilter" => Some(Box::new(tools::image_analysis::PrewittFilter::new())),
            "rangefilter" => Some(Box::new(tools::image_analysis::RangeFilter::new())),
            "removespurs" => Some(Box::new(tools::image_analysis::RemoveSpurs::new())),
            "robertscrossfilter" => {
                Some(Box::new(tools::image_analysis::RobertsCrossFilter::new()))
            }
            "scharrfilter" => Some(Box::new(tools::image_analysis::ScharrFilter::new())),
            "sigmoidalcontraststretch" => Some(Box::new(tools::image_analysis::SigmoidalContrastStretch::new())),
            "sobelfilter" => Some(Box::new(tools::image_analysis::SobelFilter::new())),
            "standarddeviationcontraststretch" => Some(Box::new(tools::image_analysis::StandardDeviationContrastStretch::new())),
            "standarddeviationfilter" => {
                Some(Box::new(tools::image_analysis::StandardDeviationFilter::new()))
            }
            "thickenrasterline" => Some(Box::new(tools::image_analysis::ThickenRasterLine::new())),
            "tophattransform" => Some(Box::new(tools::image_analysis::TophatTransform::new())),
            "totalfilter" => Some(Box::new(tools::image_analysis::TotalFilter::new())),

            // lidar_analysis
            "blockmaximum" => Some(Box::new(tools::lidar_analysis::BlockMaximum::new())),
            "blockminimum" => Some(Box::new(tools::lidar_analysis::BlockMinimum::new())),
            "flightlineoverlap" => Some(Box::new(tools::lidar_analysis::FlightlineOverlap::new())),
            "lidarelevationslice" => {
                Some(Box::new(tools::lidar_analysis::LidarElevationSlice::new()))
            }
            "lidargroundpointfilter" => {
                Some(Box::new(tools::lidar_analysis::LidarGroundPointFilter::new()))
            }
            "lidarhillshade" => Some(Box::new(tools::lidar_analysis::LidarHillshade::new())),
            "lidaridwinterpolation" => {
                Some(Box::new(tools::lidar_analysis::LidarIdwInterpolation::new()))
            }
            "lidarinfo" => Some(Box::new(tools::lidar_analysis::LidarInfo::new())),
            "lidarjoin" => Some(Box::new(tools::lidar_analysis::LidarJoin::new())),
            "lidarnearestneighbourgridding" => {
                Some(Box::new(tools::lidar_analysis::LidarNearestNeighbourGridding::new()))
            }
            "lidarpointdensity" => Some(Box::new(tools::lidar_analysis::LidarPointDensity::new())),
            "lidartile" => Some(Box::new(tools::lidar_analysis::LidarTile::new())),
            "lidartophattransform" => {
                Some(Box::new(tools::lidar_analysis::LidarTophatTransform::new()))
            }
            "normalvectors" => Some(Box::new(tools::lidar_analysis::NormalVectors::new())),

            // mathematical and statistical_analysis
            "absolutevalue" => Some(Box::new(tools::math_stat_analysis::AbsoluteValue::new())),
            "add" => Some(Box::new(tools::math_stat_analysis::Add::new())),
            "and" => Some(Box::new(tools::math_stat_analysis::And::new())),
            "arccos" => Some(Box::new(tools::math_stat_analysis::ArcCos::new())),
            "arcsin" => Some(Box::new(tools::math_stat_analysis::ArcSin::new())),
            "arctan" => Some(Box::new(tools::math_stat_analysis::ArcTan::new())),
            "atan2" => Some(Box::new(tools::math_stat_analysis::Atan2::new())),
            "ceil" => Some(Box::new(tools::math_stat_analysis::Ceil::new())),
            "cos" => Some(Box::new(tools::math_stat_analysis::Cos::new())),
            "cosh" => Some(Box::new(tools::math_stat_analysis::Cosh::new())),
            "decrement" => Some(Box::new(tools::math_stat_analysis::Decrement::new())),
            "divide" => Some(Box::new(tools::math_stat_analysis::Divide::new())),
            "equalto" => Some(Box::new(tools::math_stat_analysis::EqualTo::new())),
            "exp" => Some(Box::new(tools::math_stat_analysis::Exp::new())),
            "exp2" => Some(Box::new(tools::math_stat_analysis::Exp2::new())),
            "floor" => Some(Box::new(tools::math_stat_analysis::Floor::new())),
            "greaterthan" => Some(Box::new(tools::math_stat_analysis::GreaterThan::new())),
            "increment" => Some(Box::new(tools::math_stat_analysis::Increment::new())),
            "integerdivision" => Some(Box::new(tools::math_stat_analysis::IntegerDivision::new())),
            "isnodata" => Some(Box::new(tools::math_stat_analysis::IsNoData::new())),
            "lessthan" => Some(Box::new(tools::math_stat_analysis::LessThan::new())),
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
            "quantiles" => Some(Box::new(tools::math_stat_analysis::Quantiles::new())),
            "randomfield" => Some(Box::new(tools::math_stat_analysis::RandomField::new())),
            "rastersummarystats" => Some(Box::new(tools::math_stat_analysis::RasterSummaryStats::new())),
            "reciprocal" => Some(Box::new(tools::math_stat_analysis::Reciprocal::new())),
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
            "truncate" => Some(Box::new(tools::math_stat_analysis::Truncate::new())),
            "turningbandssimulation" => Some(Box::new(tools::math_stat_analysis::TurningBandsSimulation::new())),
            "xor" => Some(Box::new(tools::math_stat_analysis::Xor::new())),
            "zscores" => Some(Box::new(tools::math_stat_analysis::ZScores::new())),

            // stream_network_analysis
            "distancetooutlet" => {
                Some(Box::new(tools::stream_network_analysis::DistanceToOutlet::new()))
            }
            "extractstreams" => {
                Some(Box::new(tools::stream_network_analysis::ExtractStreams::new()))
            }
            "extractvalleys" => {
                Some(Box::new(tools::stream_network_analysis::ExtractValleys::new()))
            }
            "farthestchannelhead" => Some(Box::new(tools::stream_network_analysis::FarthestChannelHead::new())),
            "findmainstem" => Some(Box::new(tools::stream_network_analysis::FindMainStem::new())),
            "hackstreamorder" => {
                Some(Box::new(tools::stream_network_analysis::HackStreamOrder::new()))
            }
            "hortonstreamorder" => {
                Some(Box::new(tools::stream_network_analysis::HortonStreamOrder::new()))
            }
            "lengthofupstreamchannels" => {
                Some(Box::new(tools::stream_network_analysis::LengthOfUpstreamChannels::new()))
            }
            "removeshortstreams" => {
                Some(Box::new(tools::stream_network_analysis::RemoveShortStreams::new()))
            }
            "shrevestreammagnitude" => {
                Some(Box::new(tools::stream_network_analysis::ShreveStreamMagnitude::new()))
            }
            "strahlerstreamorder" => {
                Some(Box::new(tools::stream_network_analysis::StrahlerStreamOrder::new()))
            }
            "streamlinkclass" => {
                Some(Box::new(tools::stream_network_analysis::StreamLinkClass::new()))
            }
            "streamlinkidentifier" => {
                Some(Box::new(tools::stream_network_analysis::StreamLinkIdentifier::new()))
            }
            "streamlinklength" => {
                Some(Box::new(tools::stream_network_analysis::StreamLinkLength::new()))
            }
            "streamlinkslope" => {
                Some(Box::new(tools::stream_network_analysis::StreamLinkSlope::new()))
            }
            "streamslopecontinuous" => {
                Some(Box::new(tools::stream_network_analysis::StreamSlopeContinuous::new()))
            }
            "topologicalstreamorder" => {
                Some(Box::new(tools::stream_network_analysis::TopologicalStreamOrder::new()))
            }
            "tributaryidentifier" => {
                Some(Box::new(tools::stream_network_analysis::TributaryIdentifier::new()))
            }

            // terrain_analysis
            "aspect" => Some(Box::new(tools::terrain_analysis::Aspect::new())),
            "devfrommeanelev" => Some(Box::new(tools::terrain_analysis::DevFromMeanElev::new())),
            "difffrommeanelev" => Some(Box::new(tools::terrain_analysis::DiffFromMeanElev::new())),
            "directionalrelief" => {
                Some(Box::new(tools::terrain_analysis::DirectionalRelief::new()))
            }
            "elevabovepit" => Some(Box::new(tools::terrain_analysis::ElevAbovePit::new())),
            "elevpercentile" => Some(Box::new(tools::terrain_analysis::ElevPercentile::new())),
            "elevrelativetominmax" => Some(Box::new(tools::terrain_analysis::ElevRelativeToMinMax::new())),
            "elevrelativetowatershedminmax" => Some(Box::new(tools::terrain_analysis::ElevRelativeToWatershedMinMax::new())),
            "fetchanalysis" => Some(Box::new(tools::terrain_analysis::FetchAnalysis::new())),
            "fillmissingdata" => Some(Box::new(tools::terrain_analysis::FillMissingData::new())),
            "hillshade" => Some(Box::new(tools::terrain_analysis::Hillshade::new())),
            "horizonangle" => Some(Box::new(tools::terrain_analysis::HorizonAngle::new())),
            "maxbranchlength" => Some(Box::new(tools::terrain_analysis::MaxBranchLength::new())),
            "maxdownslopeelevchange" => Some(Box::new(tools::terrain_analysis::MaxDownslopeElevChange::new())),
            "mindownslopeelevchange" => Some(Box::new(tools::terrain_analysis::MinDownslopeElevChange::new())),
            "numdownslopeneighbours" => {
                Some(Box::new(tools::terrain_analysis::NumDownslopeNeighbours::new()))
            }
            "numupslopeneighbours" => {
                Some(Box::new(tools::terrain_analysis::NumUpslopeNeighbours::new()))
            }
            "pennocklandformclass" => Some(Box::new(tools::terrain_analysis::PennockLandformClass::new())),
            "percentelevrange" => Some(Box::new(tools::terrain_analysis::PercentElevRange::new())),
            "plancurvature" => Some(Box::new(tools::terrain_analysis::PlanCurvature::new())),
            "profilecurvature" => Some(Box::new(tools::terrain_analysis::ProfileCurvature::new())),
            "relativeaspect" => Some(Box::new(tools::terrain_analysis::RelativeAspect::new())),
            "relativestreampowerindex" => {
                Some(Box::new(tools::terrain_analysis::RelativeStreamPowerIndex::new()))
            }
            "relativetopographicposition" => {
                Some(Box::new(tools::terrain_analysis::RelativeTopographicPosition::new()))
            }
            "removeoffterrainobjects" => {
                Some(Box::new(tools::terrain_analysis::RemoveOffTerrainObjects::new()))
            }
            "ruggednessindex" => Some(Box::new(tools::terrain_analysis::RuggednessIndex::new())),  
            "sedimenttransportindex" => {
                Some(Box::new(tools::terrain_analysis::SedimentTransportIndex::new()))
            }
            "slope" => Some(Box::new(tools::terrain_analysis::Slope::new())),
            "tangentialcurvature" => {
                Some(Box::new(tools::terrain_analysis::TangentialCurvature::new()))
            }
            "totalcurvature" => Some(Box::new(tools::terrain_analysis::TotalCurvature::new())),
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
                return Err(Error::new(ErrorKind::NotFound,
                                      format!("Unrecognized tool name {}.", tool_name)))
            }
        }
    }

    pub fn tool_help(&self, tool_name: String) -> Result<(), Error> {
        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => println!("{}", get_help(tool)),
            None => {
                return Err(Error::new(ErrorKind::NotFound,
                                      format!("Unrecognized tool name {}.", tool_name)))
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
}

pub trait WhiteboxTool {
    fn get_tool_name(&self) -> String;
    fn get_tool_description(&self) -> String;
    fn get_tool_parameters(&self) -> String;
    fn get_example_usage(&self) -> String;
    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error>;
}

fn get_help<'a>(wt: Box<WhiteboxTool + 'a>) -> String {
    let tool_name = wt.get_tool_name();
    let description = wt.get_tool_description();
    let parameters = wt.get_tool_parameters();
    let example = wt.get_example_usage();
    let s: String;
    if example.len() <= 1 {
        s = format!("{} Help
Description: {}

Input parameters:
{} \n\nNo example provided",
                    tool_name,
                    description,
                    parameters);
    } else {
        s = format!("{} Help
Description: {}

Input parameters:
{}

Example usage:
{}",
                    tool_name,
                    description,
                    parameters,
                    example);
    }
    s
}

fn get_name_and_description<'a>(wt: Box<WhiteboxTool + 'a>) -> (String, String) {
    (wt.get_tool_name(), wt.get_tool_description())
}