pub mod gis_analysis;
pub mod hydro_analysis;
pub mod image_analysis;
pub mod lidar_analysis;
pub mod statistical_analysis;
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
    pub fn new<'a>(working_directory: &'a str, verbose_mode: &'a bool) -> Result<ToolManager, Error> {
        let mut tool_names = vec![];
        // gis_analysis
        tool_names.push("AverageOverlay".to_string());
        tool_names.push("BufferRaster".to_string());
        tool_names.push("Clump".to_string());
        tool_names.push("EuclideanAllocation".to_string());
        tool_names.push("EuclideanDistance".to_string());
        tool_names.push("HighestPosition".to_string());
        tool_names.push("LowestPosition".to_string());
        tool_names.push("MaxAbsoluteOverlay".to_string());
        tool_names.push("MaxOverlay".to_string());
        tool_names.push("MinAbsoluteOverlay".to_string());
        tool_names.push("MinOverlay".to_string());
        tool_names.push("PickFromList".to_string());
        tool_names.push("WeightedSum".to_string());

        // hydro_analysis
        tool_names.push("AverageUpslopeFlowpathLength".to_string());
        tool_names.push("D8FlowAccumulation".to_string());
        tool_names.push("D8Pointer".to_string());
        tool_names.push("DInfFlowAccumulation".to_string());
        tool_names.push("DInfPointer".to_string());
        tool_names.push("FD8FlowAccumulation".to_string());
        tool_names.push("JensonSnapPourPoints".to_string());
        tool_names.push("MaxUpslopeFlowpathLength".to_string());
        tool_names.push("NumInflowingNeighbours".to_string());
        tool_names.push("SnapPourPoints".to_string());
        tool_names.push("Watershed".to_string());

        // image_analysis
        tool_names.push("AdaptiveFilter".to_string());
        tool_names.push("BilateralFilter".to_string());
        tool_names.push("ConservativeSmoothingFilter".to_string());
        tool_names.push("DiffOfGaussianFilter".to_string());
        tool_names.push("GaussianFilter".to_string());
        tool_names.push("HighPassFilter".to_string());
        tool_names.push("IntegralImage".to_string());
        tool_names.push("LaplacianOfGaussianFilter".to_string());
        tool_names.push("MaximumFilter".to_string());
        tool_names.push("MeanFilter".to_string());
        tool_names.push("MinimumFilter".to_string());
        tool_names.push("NormalizedDifferenceVegetationIndex".to_string());
        tool_names.push("OlympicFilter".to_string());
        tool_names.push("PrewittFilter".to_string());
        tool_names.push("RobertsCrossFilter".to_string());
        tool_names.push("RangeFilter".to_string());
        tool_names.push("ScharrFilter".to_string());
        tool_names.push("SobelFilter".to_string());
        tool_names.push("StandardDeviationFilter".to_string());
        tool_names.push("TotalFilter".to_string());

        // lidar_analysis
        tool_names.push("FlightlineOverlap".to_string());
        tool_names.push("LidarElevationSlice".to_string());
        tool_names.push("LidarGroundPointFilter".to_string());
        tool_names.push("LidarHillshade".to_string());
        tool_names.push("LidarInfo".to_string());
        tool_names.push("LidarJoin".to_string());
        tool_names.push("LidarTile".to_string());
        tool_names.push("LidarTophatTransform".to_string());
        tool_names.push("NormalVectors".to_string());

        // statistical_analysis
        tool_names.push("Quantiles".to_string());
        tool_names.push("ZScores".to_string());

        // stream_network_analysis
        tool_names.push("FindMainStem".to_string());
        tool_names.push("HackStreamOrder".to_string());
        tool_names.push("HortonStreamOrder".to_string());
        tool_names.push("ShreveStreamMagnitude".to_string());
        tool_names.push("StrahlerStreamOrder".to_string());
        tool_names.push("StreamLinkIdentifier".to_string());
        tool_names.push("StreamLinkLength".to_string());
        tool_names.push("StreamLinkSlope".to_string());
        tool_names.push("TributaryIdentifier".to_string());

        // terrain_analysis
        tool_names.push("Aspect".to_string());
        tool_names.push("DevFromMeanElev".to_string());
        tool_names.push("DiffFromMeanElev".to_string());
        tool_names.push("ElevPercentile".to_string());
        tool_names.push("FillMissingData".to_string());
        tool_names.push("Hillshade".to_string());
        tool_names.push("NumDownslopeNeighbours".to_string());
        tool_names.push("NumUpslopeNeighbours".to_string());
        tool_names.push("PercentElevRange".to_string());
        tool_names.push("PlanCurvature".to_string());
        tool_names.push("ProfileCurvature".to_string());
        tool_names.push("RelativeAspect".to_string());
        tool_names.push("RelativeTopographicPosition".to_string());
        tool_names.push("RemoveOffTerrainObjects".to_string());
        tool_names.push("RuggednessIndex".to_string());
        tool_names.push("Slope".to_string());
        tool_names.push("TangentialCurvature".to_string());
        tool_names.push("TotalCurvature".to_string());

        tool_names.sort();
        
        let tm = ToolManager {
            working_dir: working_directory.to_string(),
            verbose: *verbose_mode,
            tool_names: tool_names,
        };
        Ok(tm)
    }

    fn get_tool(&self, tool_name: &str) -> Option<Box<WhiteboxTool+'static>> {
        match tool_name.to_lowercase().replace("_", "").as_ref() {
            // gis_analysis
            "averageoverlay" => Some(Box::new(tools::gis_analysis::AverageOverlay::new())),
            "bufferraster" => Some(Box::new(tools::gis_analysis::BufferRaster::new())),
            "clump" => Some(Box::new(tools::gis_analysis::Clump::new())),
            "euclideanallocation" => Some(Box::new(tools::gis_analysis::EuclideanAllocation::new())),
            "euclideandistance" => Some(Box::new(tools::gis_analysis::EuclideanDistance::new())),
            "highestposition" => Some(Box::new(tools::gis_analysis::HighestPosition::new())),
            "lowestposition" => Some(Box::new(tools::gis_analysis::LowestPosition::new())),
            "maxabsoluteoverlay" => Some(Box::new(tools::gis_analysis::MaxAbsoluteOverlay::new())),
            "maxoverlay" => Some(Box::new(tools::gis_analysis::MaxOverlay::new())),
            "minabsoluteoverlay" => Some(Box::new(tools::gis_analysis::MinAbsoluteOverlay::new())),
            "minoverlay" => Some(Box::new(tools::gis_analysis::MinOverlay::new())),
            "pickfromlist" => Some(Box::new(tools::gis_analysis::PickFromList::new())),
            "weightedsum" => Some(Box::new(tools::gis_analysis::WeightedSum::new())),

            // hydro_analysis
            "averageupslopeflowpathlength" => Some(Box::new(tools::hydro_analysis::AverageUpslopeFlowpathLength::new())),
            "d8flowaccumulation" => Some(Box::new(tools::hydro_analysis::D8FlowAccumulation::new())),
            "d8pointer" => Some(Box::new(tools::hydro_analysis::D8Pointer::new())),
            "dinfflowaccumulation" => Some(Box::new(tools::hydro_analysis::DInfFlowAccumulation::new())),
            "dinfpointer" => Some(Box::new(tools::hydro_analysis::DInfPointer::new())),
            "fd8flowaccumulation" => Some(Box::new(tools::hydro_analysis::FD8FlowAccumulation::new())),
            "jensonsnappourpoints" => Some(Box::new(tools::hydro_analysis::JensonSnapPourPoints::new())),
            "maxupslopeflowpathlength" => Some(Box::new(tools::hydro_analysis::MaxUpslopeFlowpathLength::new())),
            "numinflowingneighbours" => Some(Box::new(tools::hydro_analysis::NumInflowingNeighbours::new())),
            "snappourpoints" => Some(Box::new(tools::hydro_analysis::SnapPourPoints::new())),
            "watershed" => Some(Box::new(tools::hydro_analysis::Watershed::new())),

            // image_analysis
            "adaptivefilter" => Some(Box::new(tools::image_analysis::AdaptiveFilter::new())),
            "bilateralfilter" => Some(Box::new(tools::image_analysis::BilateralFilter::new())),
            "conservativesmoothingfilter" => Some(Box::new(tools::image_analysis::ConservativeSmoothingFilter::new())),
            "diffofgaussianfilter" => Some(Box::new(tools::image_analysis::DiffOfGaussianFilter::new())),
            "gaussianfilter" => Some(Box::new(tools::image_analysis::GaussianFilter::new())),
            "highpassfilter" => Some(Box::new(tools::image_analysis::HighPassFilter::new())),
            "integralimage" => Some(Box::new(tools::image_analysis::IntegralImage::new())),
            "laplacianofgaussianfilter" => Some(Box::new(tools::image_analysis::LaplacianOfGaussianFilter::new())),
            "maximumfilter" => Some(Box::new(tools::image_analysis::MaximumFilter::new())),
            "meanfilter" => Some(Box::new(tools::image_analysis::MeanFilter::new())),
            "minimumfilter" => Some(Box::new(tools::image_analysis::MinimumFilter::new())),
            "normalizeddifferencevegetationindex" => Some(Box::new(tools::image_analysis::NormalizedDifferenceVegetationIndex::new())),
            "olympicfilter" => Some(Box::new(tools::image_analysis::OlympicFilter::new())),
            "prewittfilter" => Some(Box::new(tools::image_analysis::PrewittFilter::new())),
            "rangefilter" => Some(Box::new(tools::image_analysis::RangeFilter::new())),
            "robertscrossfilter" => Some(Box::new(tools::image_analysis::RobertsCrossFilter::new())),
            "scharrfilter" => Some(Box::new(tools::image_analysis::ScharrFilter::new())),
            "sobelfilter" => Some(Box::new(tools::image_analysis::SobelFilter::new())),
            "standarddeviationfilter" => Some(Box::new(tools::image_analysis::StandardDeviationFilter::new())),
            "totalfilter" => Some(Box::new(tools::image_analysis::TotalFilter::new())),

            // lidar_analysis
            "flightlineoverlap" => Some(Box::new(tools::lidar_analysis::FlightlineOverlap::new())),
            "lidarelevationslice" => Some(Box::new(tools::lidar_analysis::LidarElevationSlice::new())),
            "lidargroundpointfilter" => Some(Box::new(tools::lidar_analysis::LidarGroundPointFilter::new())),
            "lidarhillshade" => Some(Box::new(tools::lidar_analysis::LidarHillshade::new())),
            "lidarinfo" => Some(Box::new(tools::lidar_analysis::LidarInfo::new())),
            "lidarjoin" => Some(Box::new(tools::lidar_analysis::LidarJoin::new())),
            "lidartile" => Some(Box::new(tools::lidar_analysis::LidarTile::new())),
            "lidartophattransform" => Some(Box::new(tools::lidar_analysis::LidarTophatTransform::new())),
            "normalvectors" => Some(Box::new(tools::lidar_analysis::NormalVectors::new())),

            // statistical_analysis
            "quantiles" => Some(Box::new(tools::statistical_analysis::Quantiles::new())),
            "zscores" => Some(Box::new(tools::statistical_analysis::ZScores::new())),

            // stream_network_analysis
            "findmainstem" => Some(Box::new(tools::stream_network_analysis::FindMainStem::new())),
            "hackstreamorder" => Some(Box::new(tools::stream_network_analysis::HackStreamOrder::new())),
            "hortonstreamorder" => Some(Box::new(tools::stream_network_analysis::HortonStreamOrder::new())),
            "shrevestreammagnitude" => Some(Box::new(tools::stream_network_analysis::ShreveStreamMagnitude::new())),
            "strahlerstreamorder" => Some(Box::new(tools::stream_network_analysis::StrahlerStreamOrder::new())),
            "streamlinkidentifier" => Some(Box::new(tools::stream_network_analysis::StreamLinkIdentifier::new())),
            "streamlinklength" => Some(Box::new(tools::stream_network_analysis::StreamLinkLength::new())),
            "streamlinkslope" => Some(Box::new(tools::stream_network_analysis::StreamLinkSlope::new())),
            "tributaryidentifier" => Some(Box::new(tools::stream_network_analysis::TributaryIdentifier::new())),

            // terrain_analysis
            "aspect" => Some(Box::new(tools::terrain_analysis::Aspect::new())),
            "devfrommeanelev" => Some(Box::new(tools::terrain_analysis::DevFromMeanElev::new())),
            "difffrommeanelev" => Some(Box::new(tools::terrain_analysis::DiffFromMeanElev::new())),
            "elevpercentile" => Some(Box::new(tools::terrain_analysis::ElevPercentile::new())),
            "fillmissingdata" => Some(Box::new(tools::terrain_analysis::FillMissingData::new())),
            "hillshade" => Some(Box::new(tools::terrain_analysis::Hillshade::new())),
            "numdownslopeneighbours" => Some(Box::new(tools::terrain_analysis::NumDownslopeNeighbours::new())),
            "numupslopeneighbours" => Some(Box::new(tools::terrain_analysis::NumUpslopeNeighbours::new())),
            "percentelevrange" => Some(Box::new(tools::terrain_analysis::PercentElevRange::new())),
            "plancurvature" => Some(Box::new(tools::terrain_analysis::PlanCurvature::new())),
            "profilecurvature" => Some(Box::new(tools::terrain_analysis::ProfileCurvature::new())),
            "relativeaspect" => Some(Box::new(tools::terrain_analysis::RelativeAspect::new())),
            "relativetopographicposition" => Some(Box::new(tools::terrain_analysis::RelativeTopographicPosition::new())),
            "removeoffterrainobjects" => Some(Box::new(tools::terrain_analysis::RemoveOffTerrainObjects::new())),
            "ruggednessindex" => Some(Box::new(tools::terrain_analysis::RuggednessIndex::new())),
            "slope" => Some(Box::new(tools::terrain_analysis::Slope::new())),
            "tangentialcurvature" => Some(Box::new(tools::terrain_analysis::TangentialCurvature::new())),
            "totalcurvature" => Some(Box::new(tools::terrain_analysis::TotalCurvature::new())),

            _ => None,
        }
    }

    pub fn run_tool(&self, tool_name: String, args: Vec<String>) -> Result<(), Error> {
        // if !working_dir.is_empty() {
        //     tool_args_vec.insert(0, format!("--wd={}", working_dir));
        // }

        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => return tool.run(args, &self.working_dir, self.verbose),
            None => return Err(Error::new(ErrorKind::NotFound, format!("Unrecognized tool name {}.", tool_name))),
        }
    }

    pub fn tool_help(&self, tool_name: String) -> Result<(), Error> {
        match self.get_tool(tool_name.as_ref()) {
            Some(tool) => println!("{}", get_help(tool)),
            None => return Err(Error::new(ErrorKind::NotFound, format!("Unrecognized tool name {}.", tool_name))),
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
    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error>;
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