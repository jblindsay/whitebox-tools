pub mod lidar_flightline_overlap;
pub mod lidar_info;
pub mod lidar_join;
pub mod remove_off_terrain_objects;

use tools;
use std::io::{Error, ErrorKind};

#[derive(Default)]
pub struct ToolManager {
    pub working_dir: String,
    pub verbose: bool,
}

impl ToolManager {
    pub fn new<'a>(working_directory: &'a str, verbose_mode: &'a bool) -> Result<ToolManager, Error> {
        let tm = ToolManager { working_dir: working_directory.to_string(), verbose: *verbose_mode };
        Ok(tm)
    }

    pub fn run_tool(&self, tool_name: String, args: Vec<String>) -> Result<(), Error> {
        // if !working_dir.is_empty() {
        //     tool_args_vec.insert(0, format!("--wd={}", working_dir));
        // }
        match tool_name.to_lowercase().as_ref() {
            "lidar_flightline_overlap" => {
                return tools::lidar_flightline_overlap::run(args, &self.working_dir, self.verbose);
            },
            "lidar_info" => {
            return tools::lidar_info::run(args, &self.working_dir);
            },
            "lidar_join" => {
                return tools::lidar_join::run(args, &self.working_dir, self.verbose);
            },
            "remove_off_terrain_objects" => {
                return tools::remove_off_terrain_objects::run(args, &self.working_dir, self.verbose);
            },
            _ => Err(Error::new(ErrorKind::NotFound, format!("Unrecognized tool name {}.", tool_name))),
        }
    }

    pub fn tool_help(&self, tool_name: String) -> Result<(), Error> {
        let mut description = "".to_string();
        let mut parameters = "".to_string();
        let mut example = "".to_string();
        let ret: Result<(), Error> = match tool_name.to_lowercase().as_ref() {
        "lidar_flightline_overlap" => {
            description = tools::lidar_flightline_overlap::get_tool_description();
            parameters = tools::lidar_flightline_overlap::get_tool_parameters();
            if tools::lidar_flightline_overlap::get_example_usage().is_some() {
                example = tools::lidar_flightline_overlap::get_example_usage().unwrap();
            }
            Ok(())
        },
        "lidar_info" => {
            description = tools::lidar_info::get_tool_description();
            parameters = tools::lidar_info::get_tool_parameters();
            if tools::lidar_info::get_example_usage().is_some() {
                example = tools::lidar_info::get_example_usage().unwrap();
            }
            Ok(())
        },
        "lidar_join" => {
            description = tools::lidar_join::get_tool_description();
            parameters = tools::lidar_join::get_tool_parameters();
            if tools::lidar_join::get_example_usage().is_some() {
                example = tools::lidar_join::get_example_usage().unwrap();
            }
            Ok(())
        },
        "remove_off_terrain_objects" => {
            description = tools::remove_off_terrain_objects::get_tool_description();
            parameters = tools::remove_off_terrain_objects::get_tool_parameters();
            if tools::remove_off_terrain_objects::get_example_usage().is_some() {
                example = tools::remove_off_terrain_objects::get_example_usage().unwrap();
            }
            Ok(())
        },
        _ => Err(Error::new(ErrorKind::NotFound, format!("Unrecognized tool name {}.", tool_name))),
        };
        if example.len() <= 1 {
            let s = format!("{} Help
Description: {}

Input parameters:
{} \n\nNo example provided", tool_name, description, parameters);

            println!("{}", s);
        } else {
            let s = format!("{} Help
Description: {}

Input parameters:
{}

Example usage:
{}", tool_name, description, parameters, example);

            println!("{}", s);
        }
        ret
    }

    pub fn list_tools(&self) {
        let mut tool_names = Vec::new();
        let mut tool_descriptions = Vec::new();
        tool_names.push(tools::lidar_flightline_overlap::get_tool_name());
        tool_descriptions.push(tools::lidar_flightline_overlap::get_tool_description());
        tool_names.push(tools::lidar_info::get_tool_name());
        tool_descriptions.push(tools::lidar_info::get_tool_description());
        tool_names.push(tools::lidar_join::get_tool_name());
        tool_descriptions.push(tools::lidar_join::get_tool_description());
        tool_names.push(tools::remove_off_terrain_objects::get_tool_name());
        tool_descriptions.push(tools::remove_off_terrain_objects::get_tool_description());

        let mut ret = format!("All {} Available Tools:\n", tool_names.len());
        for i in 0..tool_names.len() {
            ret.push_str(&format!("{}: {}\n\n", tool_names[i], tool_descriptions[i]));
        }

        println!("{}", ret);
    }
}
