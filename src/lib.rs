// extern crate libc;
extern crate byteorder;

pub mod io_utils;
pub mod lidar;
pub mod raster;
pub mod tools;
pub mod structures;

// use libc::{c_char};
// use std::ffi::CStr;
// use std::str;

// #[no_mangle]
// pub extern fn run_tool(tool_name: *const c_char, args_str: *const c_char) -> i32 {
//     let c_str = unsafe {
//         assert!(!tool_name.is_null());
//         CStr::from_ptr(tool_name)
//     };
//
//     let tool = c_str.to_str().unwrap().to_lowercase();
//
//     let c_str = unsafe {
//         assert!(!args_str.is_null());
//         CStr::from_ptr(args_str)
//     };
//
//     let args = c_str.to_str().unwrap();
//     let s1 = args.split("--");
//     let vec = s1.collect::<Vec<&str>>();
//     let mut out_args: Vec<String> = vec![];
//     for s in vec {
//         if s != "" {
//             let i1 = s.find("=");
//             let i2 = s.find(" ");
//             if i1 != None {
//                 let s2 = s.split("=");
//                 let vec2 = s2.collect::<Vec<&str>>();
//                 out_args.push(format!("-{}", vec2[0]));
//                 let mut s3 = "".to_owned();
//                 for i in 1..vec2.len() {
//                     if i == 1 {
//                         s3.push_str(&format!("{}", vec2[i]));
//                     } else {
//                         s3.push_str(&format!("={}", vec2[i]));
//                     }
//                 }
//                 out_args.push(s3.trim().to_owned());
//             } else if i2 != None {
//                 let s2 = s.split(" ");
//                 let vec2 = s2.collect::<Vec<&str>>();
//                 out_args.push(format!("-{}", vec2[0]));
//                 let mut s3 = "".to_owned();
//                 for i in 1..vec2.len() {
//                     s3.push_str(&format!(" {}", vec2[i]));
//                 }
//                 out_args.push(s3.trim().to_owned());
//             } else {
//                 out_args.push(format!("-{}", s));
//             }
//         }
//     }
//     println!("{:?}", out_args);
//
//     match tool.as_ref() {
//         "lidar_info" => {
//             tools::lidar_info::run(out_args);
//         },
//         _ => println!("Tool {} not recognized", tool),
//     }
//
//     0i32
// }
