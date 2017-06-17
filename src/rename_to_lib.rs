// This is an test file used to experiment with a whitebox_tools shared library (DLL).
// It is not intended for widespread use. Please note, that the file must be renamed
// 'lib.rs' and the Cargo.toml file must have a lib entry for this to work.

// extern crate libc;
// extern crate byteorder;

// pub mod io_utils;
// pub mod lidar;
// pub mod raster;
// pub mod tools;
// pub mod structures;

// use std::path;
// use std::slice;
// use std::ffi::CStr;
// use std::str;
// use libc::{size_t, c_char};
// use tools::ToolManager;

// #[no_mangle]
// pub extern fn run_tool(tool_name: *const c_char, args: *const *const c_char, length: size_t) -> i32 {
//     let c_str = unsafe {
//         assert!(!tool_name.is_null());
//         CStr::from_ptr(tool_name)
//     };

//     let tool = c_str.to_str().unwrap().to_lowercase().replace("_", "");

//     let values = unsafe { slice::from_raw_parts(args, length as usize) };
//     let strs: Vec<&str> = values.iter()
//         .map(|&p| unsafe { CStr::from_ptr(p) })  // iterator of &CStr
//         .map(|cs| cs.to_bytes())                 // iterator of &[u8]
//         .map(|bs| str::from_utf8(bs).unwrap())   // iterator of &str
//         .collect();

//     let mut args: Vec<String> = vec![];
//     for s in strs {
//         args.push(s.to_string());
//     }

//     println!("Tool Name: {}", tool);

//     let mut working_dir = String::new();
//     let sep: &str = &path::MAIN_SEPARATOR.to_string();
//     let mut verbose = false;
//     let mut tool_args_vec: Vec<String> = vec![];
//     for arg in &args {
//         println!("{}", arg);
//         if arg.starts_with("-cd") || arg.starts_with("--cd") || arg.starts_with("--wd") {
//             let mut v = arg.replace("--cd", "")
//                 .replace("--wd", "")
//                 .replace("-cd", "")
//                 .replace("\"", "")
//                 .replace("\'", "");
//             if v.starts_with("=") {
//                 v = v[1..v.len()].to_string();
//             }
//             if !v.ends_with(sep) {
//                 v.push_str(sep);
//             }
//             working_dir = v.to_string();
//         } else if arg.starts_with("-v") {
//             verbose = true;
//         } else if arg.starts_with("-") {
//             // it's an arg to be fed to the tool
//             tool_args_vec.push(arg.trim().to_string().clone());
//         }
//     }

//     let tm = ToolManager::new(&working_dir, &verbose).unwrap();
//     //let _ = tm.run_tool(tool, tool_args_vec);
//     let _ = match tm.run_tool(tool, tool_args_vec) {
//         Ok(()) => (), // do nothing
//         Err(error) => {
//             panic!("There was a problem running tool {:?}", error)
//         },
//     };


//     // let c_str = unsafe {
//     //     assert!(!args_str.is_null());
//     //     CStr::from_ptr(args_str)
//     // };
//     //
//     // let args = c_str.to_str().unwrap();
//     // let s1 = args.split("--");
//     // let vec = s1.collect::<Vec<&str>>();
//     // let mut out_args: Vec<String> = vec![];
//     // for s in vec {
//     //     if s != "" {
//     //         let i1 = s.find("=");
//     //         let i2 = s.find(" ");
//     //         if i1 != None {
//     //             let s2 = s.split("=");
//     //             let vec2 = s2.collect::<Vec<&str>>();
//     //             out_args.push(format!("-{}", vec2[0]));
//     //             let mut s3 = "".to_owned();
//     //             for i in 1..vec2.len() {
//     //                 if i == 1 {
//     //                     s3.push_str(&format!("{}", vec2[i]));
//     //                 } else {
//     //                     s3.push_str(&format!("={}", vec2[i]));
//     //                 }
//     //             }
//     //             out_args.push(s3.trim().to_owned());
//     //         } else if i2 != None {
//     //             let s2 = s.split(" ");
//     //             let vec2 = s2.collect::<Vec<&str>>();
//     //             out_args.push(format!("-{}", vec2[0]));
//     //             let mut s3 = "".to_owned();
//     //             for i in 1..vec2.len() {
//     //                 s3.push_str(&format!(" {}", vec2[i]));
//     //             }
//     //             out_args.push(s3.trim().to_owned());
//     //         } else {
//     //             out_args.push(format!("-{}", s));
//     //         }
//     //     }
//     // }
//     // println!("{:?}", out_args);

//     // match tool.as_ref() {
//     //     "lidar_info" => {
//     //         tools::lidar_info::run(args);
//     //     },
//     //     _ => println!("Tool {} not recognized", tool),
//     // }

//     0i32
// }

// // #[no_mangle]
// // pub extern fn version_info() -> i32 {
 
// // }