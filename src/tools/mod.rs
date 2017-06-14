pub mod lidar_analysis;
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
        // lidar
        tool_names.push("FlightlineOverlap".to_string());
        tool_names.push("LidarElevationSlice".to_string());
        tool_names.push("LidarGroundPointFilter".to_string());
        tool_names.push("LidarHillshade".to_string());
        tool_names.push("LidarInfo".to_string());
        tool_names.push("LidarJoin".to_string());
        tool_names.push("LidarTophatTransform".to_string());

        // terrain analysis
        tool_names.push("DevFromMeanElev".to_string());
        tool_names.push("ElevPercentile".to_string());
        tool_names.push("PercentElevRange".to_string());
        tool_names.push("RelativeTopographicPosition".to_string());
        tool_names.push("RemoveOffTerrainObjects".to_string());
        
        let tm = ToolManager {
            working_dir: working_directory.to_string(),
            verbose: *verbose_mode,
            tool_names: tool_names,
        };
        Ok(tm)
    }

    fn get_tool(&self, tool_name: &str) -> Option<Box<WhiteboxTool+'static>> {
        match tool_name.to_lowercase().replace("_", "").as_ref() {
            // lidar
            "flightlineoverlap" => Some(Box::new(tools::lidar_analysis::FlightlineOverlap::new())),
            "lidarelevationslice" => Some(Box::new(tools::lidar_analysis::LidarElevationSlice::new())),
            "lidargroundpointfilter" => Some(Box::new(tools::lidar_analysis::LidarGroundPointFilter::new())),
            "lidarhillshade" => Some(Box::new(tools::lidar_analysis::LidarHillshade::new())),
            "lidarinfo" => Some(Box::new(tools::lidar_analysis::LidarInfo::new())),
            "lidarjoin" => Some(Box::new(tools::lidar_analysis::LidarJoin::new())),
            "lidartophattransform" => Some(Box::new(tools::lidar_analysis::LidarTophatTransform::new())),

            // terrain analysis
            "devfrommeanelev" => Some(Box::new(tools::terrain_analysis::DevFromMeanElev::new())),
            "elevpercentile" => Some(Box::new(tools::terrain_analysis::ElevPercentile::new())),
            "percentelevrange" => Some(Box::new(tools::terrain_analysis::PercentElevRange::new())),
            "relativetopographicposition" => Some(Box::new(tools::terrain_analysis::RelativeTopographicPosition::new())),
            "removeoffterrainobjects" => Some(Box::new(tools::terrain_analysis::RemoveOffTerrainObjects::new())),

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