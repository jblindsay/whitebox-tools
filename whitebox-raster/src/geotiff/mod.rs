#![allow(unused_assignments, dead_code)]
pub mod geokeys;
pub mod ifd;
pub mod tiff_consts;

// use flate2::read::GzDecoder;
// use super::use_compression;
use crate::geotiff::geokeys::*;
use crate::geotiff::tiff_consts::*;
use crate::*;
use whitebox_common::spatial_ref_system::esri_wkt_from_epsg;
use whitebox_common::structures::{Point2D, PolynomialRegression2D};
use whitebox_common::utils::{ByteOrderReader, ByteOrderWriter, Endianness};
use miniz_oxide::deflate::compress_to_vec_zlib;
use miniz_oxide::inflate::decompress_to_vec_zlib;
// use libflate::zlib::Decoder;
// use libflate::deflate::Encoder;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::cmp::min;
use std::collections::HashMap;
use std::default::Default;
use std::f64;
// use std::fs;
use ifd::{Entry, Ifd};
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Error, ErrorKind, SeekFrom};
// use std::io::Read;
use std::io::Write;
use std::mem;

pub fn print_tags<'a>(file_name: &'a String) -> Result<(), Error> {
    let f = File::open(file_name.clone())?;

    //////////////////////////
    // Read the TIFF header //
    //////////////////////////

    let br = BufReader::new(f);
    let mut th = ByteOrderReader::<BufReader<File>>::new(br, Endianness::LittleEndian);

    let bo_indicator1 = th.read_u8()?;
    let bo_indicator2 = th.read_u8()?;
    let mut endian = Endianness::LittleEndian;
    if bo_indicator1 == 73 && bo_indicator2 == 73 {
        println!("Little-endian byte order used");
    } else if bo_indicator1 == 77 && bo_indicator2 == 77 {
        endian = Endianness::BigEndian;
        println!("Big-endian byte order used");
    } else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Incorrect TIFF header. Unrecognized byte-order indicator.",
        ));
    };

    if th.get_byte_order() != endian {
        th.set_byte_order(endian);
    }

    let is_big_tiff = match th.read_u16()? {
        42 => false,
        43 => true,
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect TIFF header. Unrecognized magic number.",
            ))
        }
    };

    if is_big_tiff {
        println!("BigTIFF format");
    } else {
        println!("Classic TIFF format");
    }

    if is_big_tiff && mem::size_of::<usize>() != 8 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "The BigTIFF raster format cannot be read on a 32-bit system.",
        ));
    }

    let mut ifd_offset = if !is_big_tiff {
        th.read_u32()? as usize
    } else {
        // Bytesize of offsets
        if th.read_u16()? != 8 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect BigTIFF header. Unsupported bytesize of offsets.",
            ));
        }
        // the next two bytes must be set to zero
        if th.read_u16()? != 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect BigTIFF header.",
            ));
        }
        th.read_u64()? as usize
    };

    //////////////////
    // Read the IFD //
    //////////////////

    println!("TIFF Tags:");

    let mut ifd_map = HashMap::new();
    let mut geokeys: GeoKeys = Default::default();
    let mut cur_pos: usize;
    while ifd_offset > 0 {
        th.seek(ifd_offset);
        let num_directories = if !is_big_tiff {
            th.read_u16()? as u64
        } else {
            th.read_u64()?
        };

        println!("Number of tags: {}", num_directories);

        for _ in 0..num_directories {
            let tag_id = th.read_u16()?;
            let field_type = th.read_u16()?;

            let num_values = if !is_big_tiff {
                th.read_u32()? as u64
            } else {
                th.read_u64()?
            };

            let value_offset = if !is_big_tiff {
                th.read_u32()? as u64
            } else {
                th.read_u64()?
            };

            let data_size = match field_type {
                1u16 | 2u16 | 6u16 | 7u16 => 1u64,
                3u16 | 8u16 => 2u64,
                4u16 | 9u16 | 11u16 => 4u64,
                5u16 | 10u16 | 12u16 => 8u64,
                16u16 | 17u16 | 18u16 => 8u64,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Error reading the IFDs.",
                    ))
                }
            };

            // read the tag data
            let mut data: Vec<u8> = vec![];
            if !is_big_tiff {
                if (data_size * num_values) > 4 {
                    // the values are stored at the offset location
                    cur_pos = th.pos();
                    th.seek(value_offset as usize);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                } else {
                    // the value(s) are contained in the offset
                    cur_pos = th.pos();
                    th.seek(cur_pos - 4);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                }
            } else {
                if (data_size * num_values) > 8 {
                    // the values are stored at the offset location
                    cur_pos = th.pos();
                    th.seek(value_offset as usize);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                } else {
                    // the value(s) are contained in the offset
                    cur_pos = th.pos();
                    th.seek(cur_pos - 8);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                }
            }

            let ifd = Ifd::new(tag_id, field_type, num_values, value_offset, data, endian);

            ifd_map.insert(tag_id, ifd.clone());

            println!("{}", ifd);
        }
        if !is_big_tiff {
            ifd_offset = th.read_u32()? as usize;
        } else {
            ifd_offset = th.read_u64()? as usize;
        }
    }

    match ifd_map.get(&34735) {
        Some(ifd) => geokeys.add_key_directory(&ifd.data, endian),
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The TIFF file does not contain geokeys",
            ))
        }
    };

    match ifd_map.get(&34736) {
        Some(ifd) => geokeys.add_double_params(&ifd.data, endian),
        _ => {}
    };

    match ifd_map.get(&34737) {
        Some(ifd) => geokeys.add_ascii_params(&ifd.data),
        _ => {}
    };

    println!("{}", geokeys.interpret_geokeys());

    Ok(())
}

pub fn read_geotiff<'a>(
    file_name: &'a String,
    configs: &'a mut RasterConfigs,
    data: &'a mut Vec<f64>,
) -> Result<(), Error> {
    let f = File::open(file_name.clone())?;

    //////////////////////////
    // Read the TIFF header //
    //////////////////////////
    let br = BufReader::new(f);
    let mut th = ByteOrderReader::<BufReader<File>>::new(br, configs.endian);

    let bo_indicator1 = th.read_u8()?;
    let bo_indicator2 = th.read_u8()?;
    if bo_indicator1 == 73 && bo_indicator2 == 73 {
        configs.endian = Endianness::LittleEndian;
    } else if bo_indicator1 == 77 && bo_indicator2 == 77 {
        configs.endian = Endianness::BigEndian;
    } else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Incorrect TIFF header. Unrecognized byte-order indicator.",
        ));
    }

    if th.get_byte_order() != configs.endian {
        th.set_byte_order(configs.endian);
    }

    let is_big_tiff = match th.read_u16()? {
        42 => false,
        43 => true,
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect TIFF header. Unrecognized magic number.",
            ))
        }
    };

    if is_big_tiff && mem::size_of::<usize>() != 8 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "The BigTIFF raster format cannot be read on a 32-bit system.",
        ));
    }

    let mut ifd_offset = if !is_big_tiff {
        th.read_u32()? as usize
    } else {
        // Bytesize of offsets
        if th.read_u16()? != 8 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect BigTIFF header. Unsupported bytesize of offsets.",
            ));
        }
        // the next two bytes must be set to zero
        if th.read_u16()? != 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect BigTIFF header.",
            ));
        }
        th.read_u64()? as usize
    };

    //////////////////
    // Read the IFD //
    //////////////////

    let mut ifd_map = HashMap::new();
    let mut geokeys: GeoKeys = Default::default();
    let mut cur_pos: usize;
    while ifd_offset > 0 {
        th.seek(ifd_offset);
        let num_directories = if !is_big_tiff {
            th.read_u16()? as u64
        } else {
            th.read_u64()?
        };

        for _ in 0..num_directories {
            let tag_id = th.read_u16()?;
            let field_type = th.read_u16()?;

            let num_values = if !is_big_tiff {
                th.read_u32()? as u64
            } else {
                th.read_u64()?
            };

            let value_offset = if !is_big_tiff {
                th.read_u32()? as u64
            } else {
                th.read_u64()?
            };

            let data_size = match field_type {
                1u16 | 2u16 | 6u16 | 7u16 => 1u64,
                3u16 | 8u16 => 2u64,
                4u16 | 9u16 | 11u16 => 4u64,
                5u16 | 10u16 | 12u16 => 8u64,
                16u16 | 17u16 | 18u16 => 8u64,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Error reading the IFDs.",
                    ))
                }
            };

            // read the tag data
            let mut data: Vec<u8> = vec![];
            if !is_big_tiff {
                if (data_size * num_values) > 4 {
                    // the values are stored at the offset location
                    cur_pos = th.pos();
                    th.seek(value_offset as usize);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                } else {
                    // the value(s) are contained in the offset
                    cur_pos = th.pos();
                    th.seek(cur_pos - 4);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                }
            } else {
                if (data_size * num_values) > 8 {
                    // the values are stored at the offset location
                    cur_pos = th.pos();
                    th.seek(value_offset as usize);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                } else {
                    // the value(s) are contained in the offset
                    cur_pos = th.pos();
                    th.seek(cur_pos - 8);
                    for _ in 0..num_values * data_size {
                        data.push(th.read_u8()?);
                    }
                    th.seek(cur_pos);
                }
            }

            let ifd = Ifd::new(
                tag_id,
                field_type,
                num_values,
                value_offset,
                data,
                configs.endian,
            );

            ifd_map.insert(tag_id, ifd.clone());
        }
        // WhiteboxTools currently only supports single-band rasters.
        // Sometimes GeoTIFF contain multiple bands. When this is the case,
        // only the first band should be read. This is often the case when
        // users have used pyramiding on their file. To get the tags of the
        // other bands, uncomment the code below; however, doing so may
        // cause erratic behaviour of certain tools. e.g. see issue # 102
        // clip_raster_to_polygon issue

        // if !is_big_tiff {
        //     ifd_offset = th.read_u32()? as usize;
        // } else {
        //     ifd_offset = th.read_u64()? as usize;
        // }

        ifd_offset = 0; // comment this out if you want to read additional images.
    }

    configs.columns = match ifd_map.get(&256) {
        Some(ifd) => {
            // The 256 tag can be either u16 or u32 type
            if ifd.ifd_type == 3 {
                ifd.interpret_as_u16()[0] as usize
            } else {
                ifd.interpret_as_u32()[0] as usize
            }
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The raster Columns value was not read correctly",
            ))
        }
    };

    configs.rows = match ifd_map.get(&257) {
        Some(ifd) => {
            // The 257 tag can be either u16 or u32 type
            if ifd.ifd_type == 3 {
                ifd.interpret_as_u16()[0] as usize
            } else {
                ifd.interpret_as_u32()[0] as usize
            }
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The raster Rows value was not read correctly",
            ))
        }
    };

    let bits_per_sample = match ifd_map.get(&258) {
        Some(ifd) => ifd.interpret_as_u16(),
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The raster BitsPerSample value was not read correctly",
            ))
        }
    };

    let compression = match ifd_map.get(&259) {
        Some(ifd) => ifd.interpret_as_u16()[0],
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The raster Compression method value was not read correctly",
            ))
        }
    };

    if compression != COMPRESS_NONE
        && compression != COMPRESS_PACKBITS
        && compression != COMPRESS_LZW
        && compression != COMPRESS_DEFLATE
    {
        println!("Compression: {}", compression);
        return Err(Error::new(
            ErrorKind::InvalidData,
            "The WhiteboxTools GeoTIFF decoder currently only supports PACKBITS, LZW, and DEFLATE compression.",
        ));
    }

    let photometric_interp = match ifd_map.get(&262) {
        Some(ifd) => ifd.interpret_as_u16()[0],
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The raster PhotometricInterpretation value was not read correctly",
            ))
        }
    };

    // let num_samples = match ifd_map.get(&277) {
    //     Some(ifd) => ifd.interpret_as_u16()[0],
    //     _ => 0,
    // };

    match ifd_map.get(&280) {
        Some(ifd) => {
            configs.display_min = ifd.interpret_as_u16()[0] as f64;
        }
        _ => {}
    };

    match ifd_map.get(&281) {
        Some(ifd) => {
            configs.display_max = ifd.interpret_as_u16()[0] as f64;
        }
        _ => {}
    };

    let extra_samples = match ifd_map.get(&338) {
        Some(ifd) => ifd.interpret_as_u16()[0],
        _ => 0,
    };

    let sample_format = match ifd_map.get(&339) {
        Some(ifd) => ifd.interpret_as_u16(),
        _ => [0].to_vec(),
    };

    configs.nodata = match ifd_map.get(&TAG_GDAL_NODATA) {
        Some(ifd) => {
            let s = ifd.interpret_as_ascii().trim().to_string();
            if bits_per_sample[0] == 32 && sample_format[0] == 3 {
                s.parse::<f32>().unwrap_or(-32768f32) as f64
            } else {
                s.parse::<f64>().unwrap_or(-32768f64)
            }
        }
        _ => -32768f64,
    };

    // GeoKeyDirectoryTag
    match ifd_map.get(&34735) {
        Some(ifd) => {
            configs.geo_key_directory = ifd.interpret_as_u16();
            geokeys.add_key_directory(&ifd.data, configs.endian);
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The TIFF file does not contain geokeys",
            ))
        }
    };

    // GeoDoubleParamsTag
    match ifd_map.get(&34736) {
        Some(ifd) => {
            configs.geo_double_params = ifd.interpret_as_f64();
            geokeys.add_double_params(&ifd.data, configs.endian);
        }
        _ => {}
    };

    // GeoAsciiParamsTag
    match ifd_map.get(&34737) {
        Some(ifd) => {
            configs.geo_ascii_params = ifd.interpret_as_ascii();
            geokeys.add_ascii_params(&ifd.data);
        }
        _ => {}
    };

    let geokeys_map = geokeys.get_ifd_map(configs.endian);

    // ModelTiePointTag
    configs.model_tiepoint = match ifd_map.get(&33922) {
        Some(ifd) => ifd.interpret_as_f64(),
        _ => vec![0.0],
    };

    // ModelPixelScale
    match ifd_map.get(&33550) {
        Some(ifd) => {
            let vals = ifd.interpret_as_f64();
            if vals.len() != 3 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error: the ModelPixelScaleTag (33550) is not specified correctly in the GeoTIFF file.",
                ));
            }
            for i in 0..3 {
                configs.model_pixel_scale[i] = vals[i];
            }
        }
        _ => {}
    }

    // ModelTransformationTag
    match ifd_map.get(&33920) {
        Some(ifd) => {
            let vals = ifd.interpret_as_f64();
            if vals.len() != 16 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error: the ModelTransformationTag (33920) is not specified correctly in the GeoTIFF file.",
                ));
            }
            for i in 0..16 {
                configs.model_transformation[i] = vals[i];
            }
        }
        _ => {}
    }
    match ifd_map.get(&34264) {
        Some(ifd) => {
            let vals = ifd.interpret_as_f64();
            if vals.len() != 16 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error: the ModelTransformationTag (34264) is not specified correctly in the GeoTIFF file.",
                ));
            }
            for i in 0..16 {
                configs.model_transformation[i] = vals[i];
            }
        }
        _ => {}
    }

    if configs.model_tiepoint.len() == 6 {
        // see if the model_pixel_scale tag was actually specified
        if configs.model_pixel_scale[0] == 0.0 {
            configs.model_pixel_scale[0] = 1.0;
            configs.model_pixel_scale[1] = 1.0;
            println!("Warning: The ModelPixelScaleTag (33550) is not specified. A pixel resolution of 1.0 has been assumed.");
        }
        // The common case of one tie-point and pixel size.
        // The position and scale of the data is known exactly, and
        // no rotation or shearing is needed to fit into the model space.
        // Use the ModelTiepointTag to define the (X,Y,Z) coordinates
        // of the known raster point, and the ModelPixelScaleTag to
        // specify the scale.
        configs.resolution_x = configs.model_pixel_scale[0];
        configs.resolution_y = configs.model_pixel_scale[1];
        let tx = configs.model_tiepoint[3] - configs.model_tiepoint[0] / configs.resolution_x;
        let ty = configs.model_tiepoint[4] + configs.model_tiepoint[1] / configs.resolution_y;
        // upper-left corner coordinates
        let mut col = 0.0;
        let mut row = 0.0;
        configs.west = configs.resolution_x * col + tx;
        configs.north = -configs.resolution_y * row + ty;
        // lower-right corner coordinates
        col = (configs.columns - 1) as f64;
        row = (configs.rows - 1) as f64;
        configs.east = configs.resolution_x * col + tx;
        configs.south = -configs.resolution_y * row + ty;
    } else if configs.model_tiepoint.len() > 6 {
        // how many points are there?
        let num_tie_points = configs.model_tiepoint.len() / 6;
        let mut x = Vec::with_capacity(num_tie_points);
        let mut y = Vec::with_capacity(num_tie_points);
        let mut x_prime = Vec::with_capacity(num_tie_points);
        let mut y_prime = Vec::with_capacity(num_tie_points);
        for a in 0..num_tie_points {
            let b = a * 6;
            x.push(configs.model_tiepoint[b]);
            y.push(configs.model_tiepoint[b + 1]);
            x_prime.push(configs.model_tiepoint[b + 3]);
            y_prime.push(configs.model_tiepoint[b + 4]);
        }

        let mut minxp = std::f64::MAX;
        for i in 0..num_tie_points {
            if x_prime[i] < minxp {
                minxp = x_prime[i];
            }
        }
        for i in 0..num_tie_points {
            x_prime[i] -= minxp;
        }

        let mut minyp = std::f64::MAX;
        for i in 0..num_tie_points {
            if y_prime[i] < minyp {
                minyp = y_prime[i];
            }
        }
        for i in 0..num_tie_points {
            y_prime[i] -= minyp;
        }

        let poly_order = 3;
        let pr2d = PolynomialRegression2D::new(poly_order, &x_prime, &y_prime, &x, &y).unwrap();

        // upper-left corner coordinates
        let mut col = 0.0f64;
        let mut row = 0.0f64;
        let mut val = pr2d.get_value(col, row);
        let upper_left_x = minxp + val.0;
        let upper_left_y = minyp + val.1;

        // upper-right corner coordinates
        col = (configs.columns - 1) as f64;
        row = 0.0;
        val = pr2d.get_value(col, row);
        let upper_right_x = minxp + val.0;
        let upper_right_y = minyp + val.1;

        // lower-left corner coordinates
        col = 0.0;
        row = (configs.rows - 1) as f64;
        val = pr2d.get_value(col, row);
        let lower_left_x = minxp + val.0;
        let lower_left_y = minyp + val.1;

        // lower-right corner coordinates
        col = (configs.columns - 1) as f64;
        row = (configs.rows - 1) as f64;
        val = pr2d.get_value(col, row);
        let lower_right_x = minxp + val.0;
        let lower_right_y = minyp + val.1;

        configs.west = lower_left_x.min(upper_left_x);
        configs.east = lower_right_x.max(upper_right_x);
        configs.south = lower_left_y.min(lower_right_y);
        configs.north = upper_left_y.max(upper_right_y);

        // resolution
        let upper_right = Point2D::new(upper_right_x, upper_right_y);
        let upper_left = Point2D::new(upper_left_x, upper_left_y);
        let lower_left = Point2D::new(lower_left_x, lower_left_y);
        configs.resolution_x = upper_right.distance(&upper_left) / configs.columns as f64;
        configs.resolution_y = upper_left.distance(&lower_left) / configs.rows as f64;
    } else if configs.model_transformation[0] != 0.0 {
        configs.resolution_x = configs.model_transformation[0];
        configs.resolution_y = configs.model_transformation[5].abs();
        // upper-left corner coordinates
        let mut col = 0.0;
        let mut row = 0.0;
        let upper_left_x = configs.model_transformation[0] * col
            + configs.model_transformation[1] * row
            + configs.model_transformation[3];
        let upper_left_y = configs.model_transformation[4] * col
            + configs.model_transformation[5] * row
            + configs.model_transformation[7];

        // upper-right corner coordinates
        col = (configs.columns - 1) as f64;
        row = 0.0;
        let upper_right_x = configs.model_transformation[0] * col
            + configs.model_transformation[1] * row
            + configs.model_transformation[3];
        let upper_right_y = configs.model_transformation[4] * col
            + configs.model_transformation[5] * row
            + configs.model_transformation[7];

        // lower-left corner coordinates
        col = 0.0;
        row = (configs.rows - 1) as f64;
        let lower_left_x = configs.model_transformation[0] * col
            + configs.model_transformation[1] * row
            + configs.model_transformation[3];
        let lower_left_y = configs.model_transformation[4] * col
            + configs.model_transformation[5] * row
            + configs.model_transformation[7];

        // lower-right corner coordinates
        col = (configs.columns - 1) as f64;
        row = (configs.rows - 1) as f64;
        let lower_right_x = configs.model_transformation[0] * col
            + configs.model_transformation[1] * row
            + configs.model_transformation[3];
        let lower_right_y = configs.model_transformation[4] * col
            + configs.model_transformation[5] * row
            + configs.model_transformation[7];

        configs.west = lower_left_x.min(upper_left_x);
        configs.east = lower_right_x.max(upper_right_x);
        configs.south = lower_left_y.min(lower_right_y);
        configs.north = upper_left_y.max(upper_right_y);
    } else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "The model-space/raster-space transformation cannot be defined.",
        ));
    }

    // Get the EPSG code and WKT CRS
    configs.epsg_code = geokeys.find_epsg_code();
    configs.coordinate_ref_system_wkt = esri_wkt_from_epsg(configs.epsg_code);
    // if geokeys_map.contains_key(&2048) {
    //     // geographic coordinate system
    //     configs.epsg_code = geokeys_map.get(&2048).unwrap().interpret_as_u16()[0];
    // } else if geokeys_map.contains_key(&3072) {
    //     // projected coordinate system
    //     configs.epsg_code = geokeys_map.get(&3072).unwrap().interpret_as_u16()[0];
    // }

    // Determine the image mode.
    let kw_map = get_keyword_map();
    let photomet_map = kw_map.get(&262).unwrap();
    let photomet_str: String = photomet_map.get(&photometric_interp).unwrap().to_string();
    // let mode: ImageMode;
    let mode: u16;
    let mut palette = vec![];
    if photomet_str == "RGB" {
        configs.photometric_interp = PhotometricInterpretation::RGB;
        if bits_per_sample[0] == 16 {
            if bits_per_sample[1] != 16 || bits_per_sample[2] != 16 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Wrong number of samples for 16bit RGB.",
                ));
            }
        } else {
            if bits_per_sample[0] != 8 || bits_per_sample[1] != 8 || bits_per_sample[2] != 8 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Wrong number of samples for 8bit RGB.",
                ));
            }
        }
        // RGB images normally have 3 samples per pixel.
        // If there are more, ExtraSamples (p. 31-32 of the spec)
        // gives their meaning (usually an alpha channel).
        // This implementation does not support extra samples
        // of an unspecified type.
        mode = match bits_per_sample.len() {
            3 => IM_RGB, //ImageMode::RGB,
            4 => {
                match extra_samples {
                    // Not sure why, but some GeoTIFFs have extra samples = 0 even though they are clearly RGBA
                    0 | 1 => IM_RGBA, // ImageMode::RGBA,
                    2 => IM_NRGBA,    //ImageMode::NRGBA,
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "Wrong number of samples for RGB.",
                        ))
                    }
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Wrong number of samples for RGB.",
                ))
            }
        };
    } else if photomet_str == "Paletted" {
        configs.photometric_interp = PhotometricInterpretation::Categorical;
        mode = IM_PALETTED; //ImageMode::Paletted;
                            // retrieve the palette colour data
        let color_map = match ifd_map.get(&320) {
            Some(ifd) => ifd.interpret_as_u16(),
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Colour map not present in Paletted TIFF.",
                ))
            }
        };
        let num_colors = color_map.len() / 3;
        if color_map.len() % 3 != 0 || num_colors <= 0 || num_colors > 256 {
            return Err(Error::new(ErrorKind::InvalidData, "bad ColorMap length"));
        }
        for i in 0..num_colors {
            // colours in the colour map are given in 16-bit channels
            // and need to be rescaled to an 8-bit format.
            let red = (color_map[i] as f64 / 65535.0 * 255.0) as u32;
            let green = (color_map[i + num_colors] as f64 / 65535.0 * 255.0) as u32;
            let blue = (color_map[i + 2 * num_colors] as f64 / 65535.0 * 255.0) as u32;
            let a = 255u32;
            let val = ((a << 24) | (red << 16) | (green << 8) | blue) as u32;
            palette.push(val);
        }
    } else if photomet_str == "WhiteIsZero" {
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        mode = IM_GRAYINVERT; //ImageMode::GrayInvert;
    } else if photomet_str == "BlackIsZero" {
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        mode = IM_GRAY; //ImageMode::Gray;
    } else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Unsupported image format.",
        ));
    }

    let width = configs.columns;
    let height = configs.rows;

    let mut block_padding = false;
    let mut block_width = configs.columns;
    let block_height; // = configs.rows;
    let mut blocks_across = 1;
    let blocks_down; // = 1;

    let block_offsets: Vec<u64>; //  = vec![];
    let block_counts: Vec<u64>; // = vec![];

    if ifd_map.contains_key(&322) {
        // it's a tile oriented file.
        block_padding = true;

        block_width = match ifd_map.get(&322) {
            Some(ifd) => {
                // The 322 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    ifd.interpret_as_u16()[0] as usize
                } else {
                    ifd.interpret_as_u32()[0] as usize
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The TileWidth value was not read correctly",
                ))
            }
        };

        block_height = match ifd_map.get(&323) {
            Some(ifd) => {
                // The 323 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    ifd.interpret_as_u16()[0] as usize
                } else {
                    ifd.interpret_as_u32()[0] as usize
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The TileLength value was not read correctly",
                ))
            }
        };

        blocks_across = (width + block_width - 1) / block_width;
        blocks_down = (height + block_height - 1) / block_height;

        block_offsets = match ifd_map.get(&324) {
            Some(ifd) => {
                if ifd.ifd_type == 3 {
                    let ifd_data = ifd.interpret_as_u16();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 4 {
                    let ifd_data = ifd.interpret_as_u32();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 16 {
                    ifd.interpret_as_u64()
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "The raster TileOffsets values were not read correctly",
                    ));
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The raster TileOffsets value was not read correctly",
                ))
            }
        };

        block_counts = match ifd_map.get(&325) {
            Some(ifd) => {
                // The 325 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    let ifd_data = ifd.interpret_as_u16();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 4 {
                    let ifd_data = ifd.interpret_as_u32();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 16 {
                    ifd.interpret_as_u64()
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "The raster TileLength values were not read correctly",
                    ));
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The TileLength values were not read correctly",
                ))
            }
        };
    } else {
        block_height = match ifd_map.get(&278) {
            Some(ifd) => {
                // The 278 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    ifd.interpret_as_u16()[0] as usize
                } else {
                    ifd.interpret_as_u32()[0] as usize
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The RowsPerStrip value was not read correctly",
                ))
            }
        };

        blocks_down = (height + block_height - 1) / block_height;

        block_offsets = match ifd_map.get(&273) {
            Some(ifd) => {
                // The 273 tag can be either u16, u32, or u64 (BigTIFF) type
                if ifd.ifd_type == 3 {
                    let ifd_data = ifd.interpret_as_u16();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 4 {
                    let ifd_data = ifd.interpret_as_u32();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 16 {
                    ifd.interpret_as_u64()
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "The raster StripOffsets values were not read correctly",
                    ));
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The raster StripOffsets values were not read correctly",
                ))
            }
        };

        block_counts = match ifd_map.get(&279) {
            Some(ifd) => {
                // The 279 tag can be either u16, u32, or u64 (BigTIFF) type
                if ifd.ifd_type == 3 {
                    let ifd_data = ifd.interpret_as_u16();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 4 {
                    let ifd_data = ifd.interpret_as_u32();
                    let mut ret: Vec<u64> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u64);
                    }
                    ret
                } else if ifd.ifd_type == 16 {
                    ifd.interpret_as_u64()
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "The raster StripByteCounts value was not read correctly",
                    ));
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The raster StripByteCounts value was not read correctly",
                ))
            }
        };
    }

    ////////////////////
    // Read the data! //
    ////////////////////
    if data.len() > 0 {
        data.clear();
    }
    data.reserve_exact(configs.rows * configs.columns);
    unsafe {
        // The memory will be initialized when we read
        // the pixel values.
        data.set_len(configs.rows * configs.columns);
    }

    for i in 0..blocks_across {
        let mut blk_w = block_width;
        if !block_padding && i == blocks_across - 1 && width % block_width != 0 {
            blk_w = width % block_width;
        }
        for j in 0..blocks_down {
            let mut blk_h = block_height;
            if !block_padding && j == blocks_down - 1 && height % block_height != 0 {
                blk_h = height % block_height;
            }
            let offset = block_offsets[j * blocks_across + i] as usize;
            let n = block_counts[j * blocks_across + i] as usize;
            let mut buf: Vec<u8> = vec![];
            if n != 0 {
                // it's not a sparse tile
                match compression {
                    COMPRESS_NONE => {
                        // no compression
                        // buf = vec![0u8; n];
                        buf.reserve_exact(n);
                        unsafe { buf.set_len(n); }
                        th.seek(offset);
                        th.read_exact(&mut buf)?;
                    }
                    COMPRESS_PACKBITS => {
                        // buf = packbits_decoder(th.buffer[offset..(offset + n)].to_vec());
                        let mut b = vec![0u8; n];
                        th.seek(offset);
                        th.read_exact(&mut b).expect("Error reading bytes from file.");
                        buf = packbits_decoder(b);
                    }
                    COMPRESS_LZW => {
                        let mut compressed = vec![0; n];
                        th.seek(offset);
                        th.read_exact(&mut compressed).expect("Error reading bytes from file.");
                        let max_uncompressed_length = block_width * block_height * bits_per_sample.len() * bits_per_sample[0] as usize / 8;
                        buf = Vec::with_capacity(max_uncompressed_length);
                        let mut decoder = lzw::DecoderEarlyChange::new(lzw::MsbReader::new(), 8);
                        let mut bytes_read = 0;
                        while bytes_read < n && buf.len() < max_uncompressed_length {
                            let (len, bytes) = decoder.decode_bytes(&compressed[bytes_read..]).expect("Error encountered while decoding the LZW compressed GeoTIFF file.");
                            bytes_read += len;
                            buf.extend_from_slice(bytes);
                        }
                    }
                    COMPRESS_DEFLATE => {
                        // let mut dec = GzDecoder::new(th.buffer[offset..(offset + n)].to_vec());
                        // let compressed = &th.buffer[offset..(offset + n)];
                        // let mut decoder = Decoder::new(&compressed[..]).unwrap();
                        // decoder.read_to_end(&mut buf).unwrap();
                        th.seek(offset);
                        let mut compressed = vec![0u8; n];
                        th.read_exact(&mut compressed).expect("Error reading bytes from file.");
                        // let mut decoder = Decoder::new(&compressed[..])?;
                        // decoder.read_to_end(&mut buf).unwrap();
                        buf.extend(decompress_to_vec_zlib(&compressed).expect("Error encountered while decoding the DEFLATE compressed GeoTIFF file."));
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "The WhiteboxTools GeoTIFF decoder currently only supports PACKBITS and DEFLATE compression.",
                        ))
                    }
                }
            }

            // println!("{:?}", &buf[0..8]);
            let mut bor = ByteOrderReader::<Cursor<Vec<u8>>>::new(Cursor::new(buf), configs.endian);

            let xmin = i * block_width;
            let ymin = j * block_height;
            let mut xmax = xmin + blk_w;
            let mut ymax = ymin + blk_h;

            xmax = min(xmax, width);
            ymax = min(ymax, height);

            let skip_bytes = if xmin + blk_w > width {
                xmin + blk_w - width
            } else {
                0
            };

            let mut off = 0;
            let mut i: usize;
            let (mut red, mut green, mut blue): (u32, u32, u32);
            if n != 0 {
                match mode {
                    IM_GRAYINVERT | IM_GRAY => {
                        match sample_format[0] {
                            1 => {
                                // unsigned integer
                                match bits_per_sample[0] {
                                    8 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_u8()? as f64;
                                                    off += 1;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes);
                                            }
                                        }
                                    }
                                    16 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_u16()? as f64;
                                                    off += 2;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 2);
                                            }
                                        }
                                    }
                                    32 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_u32()? as f64;
                                                    off += 4;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 4);
                                            }
                                        }
                                    }
                                    64 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_u64()? as f64;
                                                    off += 8;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 8);
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(Error::new(
                                            ErrorKind::InvalidData,
                                            "The raster was not read correctly",
                                        ))
                                    }
                                }
                            }
                            2 => {
                                // signed integer
                                match bits_per_sample[0] {
                                    8 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_i8()? as f64;
                                                    off += 1;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes);
                                            }
                                        }
                                    }
                                    16 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_i16()? as f64;
                                                    off += 2;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 2);
                                            }
                                        }
                                    }
                                    32 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_i32()? as f64;
                                                    off += 4;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 4);
                                            }
                                        }
                                    }
                                    64 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_i64()? as f64;
                                                    off += 8;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 8);
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(Error::new(
                                            ErrorKind::InvalidData,
                                            "The raster was not read correctly",
                                        ))
                                    }
                                }
                            }
                            3 => {
                                // floating point
                                match bits_per_sample[0] {
                                    32 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                i = y * width + x;
                                                data[i] = bor.read_f32()? as f64;
                                                off += 4;
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 4);
                                            }
                                        }
                                    }
                                    64 => {
                                        for y in ymin..ymax {
                                            for x in xmin..xmax {
                                                if off <= bor.len() {
                                                    i = y * width + x;
                                                    data[i] = bor.read_f64()?;
                                                    off += 8;
                                                }
                                            }
                                            if skip_bytes > 0 {
                                                bor.inc_pos(skip_bytes * 8);
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err(Error::new(
                                            ErrorKind::InvalidData,
                                            "The raster was not read correctly",
                                        ))
                                    }
                                }
                            }
                            _ => {
                                return Err(Error::new(
                                    ErrorKind::InvalidData,
                                    "The raster was not read correctly",
                                ))
                            }
                        }
                    }
                    IM_PALETTED => {
                        let mut value: usize;
                        for y in ymin..ymax {
                            for x in xmin..xmax {
                                i = y * width + x;
                                value = bor.read_u8()? as usize;
                                data[i] = palette[value] as f64;
                            }
                        }
                    }
                    IM_RGB => {
                        let mut value: u32;
                        let mut a: u32;
                        if bits_per_sample[0] == 8 {
                            for y in ymin..ymax {
                                for x in xmin..xmax {
                                    red = bor.read_u8()? as u32; //uint32(g.buf[g.off]);
                                    green = bor.read_u8()? as u32; //uint32(g.buf[g.off+1]);
                                    blue = bor.read_u8()? as u32; //uint32(g.buf[g.off+2]);
                                    a = 255u32;
                                    value = (a << 24) | (blue << 16) | (green << 8) | red;
                                    i = y * width + x;
                                    data[i] = value as f64;
                                }
                            }
                        } else if bits_per_sample[0] == 16 {
                            // the spec doesn't talk about 16-bit RGB images so
                            // I'm not sure why I bother with this. They specifically
                            // say that RGB images are 8-bits per channel. Anyhow,
                            // I rescale the 16-bits to an 8-bit channel for simplicity.
                            let mut value: u32;
                            let mut a: u32;
                            for y in ymin..ymax {
                                for x in xmin..xmax {
                                    red = (bor.read_u16()? as f64 / 65535f64 * 255f64) as u32;
                                    green = (bor.read_u16()? as f64 / 65535f64 * 255f64) as u32;
                                    blue = (bor.read_u16()? as f64 / 65535f64 * 255f64) as u32;
                                    a = 255u32;
                                    value = (a << 24) | (blue << 16) | (green << 8) | red;
                                    i = y * width + x;
                                    data[i] = value as f64;
                                }
                            }
                        } else {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                "The raster was not read correctly",
                            ));
                        }
                    }
                    IM_NRGBA | IM_RGBA => {
                        let mut value: u32;
                        let mut a: u32;
                        if bits_per_sample[0] == 8 {
                            for y in ymin..ymax {
                                for x in xmin..xmax {
                                    red = bor.read_u8()? as u32; //uint32(g.buf[g.off]);
                                    green = bor.read_u8()? as u32; //uint32(g.buf[g.off+1]);
                                    blue = bor.read_u8()? as u32; //uint32(g.buf[g.off+2]);
                                    a = bor.read_u8()? as u32;
                                    value = (a << 24) | (blue << 16) | (green << 8) | red;
                                    i = y * width + x;
                                    data[i] = value as f64;
                                }
                            }
                        } else if bits_per_sample[0] == 16 {
                            // the spec doesn't talk about 16-bit RGB images so
                            // I'm not sure why I bother with this. They specifically
                            // say that RGB images are 8-bits per channel. Anyhow,
                            // I rescale the 16-bits to an 8-bit channel for simplicity.
                            let mut value: u32;
                            let mut a: u32;
                            for y in ymin..ymax {
                                for x in xmin..xmax {
                                    red = (bor.read_u16()? as f64 / 65535f64 * 255f64) as u32;
                                    green = (bor.read_u16()? as f64 / 65535f64 * 255f64) as u32;
                                    blue = (bor.read_u16()? as f64 / 65535f64 * 255f64) as u32;
                                    a = (bor.read_u16()? as f64 / 65535f64 * 255f64) as u32;
                                    value = (a << 24) | (blue << 16) | (green << 8) | red;
                                    i = y * width + x;
                                    data[i] = value as f64;
                                }
                            }
                        } else {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                "The raster was not read correctly",
                            ));
                        }
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "The raster was not read correctly",
                        ))
                    }
                }
            } else {
                // GDAL supports sparse tiles. That is, if the block count is zero,
                // instead of reading the block, simply assume it is filled with either
                // nodata, if the value is defined, or zeros otherwise.
                for y in ymin..ymax {
                    for x in xmin..xmax {
                        i = y * width + x;
                        data[i] = configs.nodata;
                    }
                }
            }

            match mode {
                IM_GRAYINVERT | IM_GRAY => {
                    //ImageMode::GrayInvert | ImageMode::Gray => {
                    configs.photometric_interp = PhotometricInterpretation::Continuous;
                    match sample_format[0] {
                        1 => {
                            // unsigned integer
                            match bits_per_sample[0] {
                                8 => {
                                    configs.data_type = DataType::U8;
                                }
                                16 => {
                                    configs.data_type = DataType::U16;
                                }
                                32 => {
                                    configs.data_type = DataType::U32;
                                }
                                64 => {
                                    configs.data_type = DataType::U64;
                                }
                                _ => {
                                    return Err(Error::new(
                                        ErrorKind::InvalidData,
                                        "The raster was not read correctly",
                                    ))
                                }
                            }
                        }
                        2 => {
                            // signed integer
                            match bits_per_sample[0] {
                                8 => {
                                    configs.data_type = DataType::I8;
                                }
                                16 => {
                                    configs.data_type = DataType::I16;
                                }
                                32 => {
                                    configs.data_type = DataType::I32;
                                }
                                64 => {
                                    configs.data_type = DataType::I64;
                                }
                                _ => {
                                    return Err(Error::new(
                                        ErrorKind::InvalidData,
                                        "The raster was not read correctly",
                                    ))
                                }
                            }
                        }
                        3 => {
                            // floating point
                            match bits_per_sample[0] {
                                32 => {
                                    configs.data_type = DataType::F32;
                                }
                                64 => {
                                    configs.data_type = DataType::F64;
                                }
                                _ => {
                                    return Err(Error::new(
                                        ErrorKind::InvalidData,
                                        "The raster was not read correctly",
                                    ))
                                }
                            }
                        }
                        _ => {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                "The raster was not read correctly",
                            ))
                        }
                    }
                }
                IM_PALETTED => {
                    //ImageMode::Paletted => {
                    configs.photometric_interp = PhotometricInterpretation::Categorical;
                    configs.data_type = DataType::U8;
                }
                IM_RGB => {
                    configs.photometric_interp = PhotometricInterpretation::RGB;
                    if bits_per_sample[0] == 8 {
                        configs.data_type = DataType::U8;
                    } else if bits_per_sample[0] == 16 {
                        configs.data_type = DataType::U16;
                    } else {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "The raster was not read correctly",
                        ));
                    }
                }
                IM_NRGBA | IM_RGBA => {
                    // if bits_per_sample[0] == 8 {
                    //     configs.data_type = DataType::U32;
                    // } else if bits_per_sample[0] == 16 {
                    //     configs.data_type = DataType::U64;
                    // } else {
                    //     return Err(Error::new(
                    //         ErrorKind::InvalidData,
                    //         "The raster was not read correctly",
                    //     ));
                    // }
                    if bits_per_sample[0] == 8 && bits_per_sample.len() == 4 {
                        configs.data_type = DataType::RGBA32;
                    } else if bits_per_sample[0] == 8 && bits_per_sample.len() == 3 {
                        configs.data_type = DataType::RGB24;
                    } else if bits_per_sample[0] == 16 {
                        configs.data_type = DataType::U16;
                    } else {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "The raster was not read correctly",
                        ));
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "The raster was not read correctly",
                    ))
                }
            }
        }
    }

    // Check to see if a predictor is used with LZW and DEFLATE
    match ifd_map.get(&317) {
        Some(ifd) => {
            if ifd.interpret_as_u16()[0] == 2 {
                // Horizontal predictor
                // transform the data
                let mut idx: usize;
                for row in 0..configs.rows {
                    for col in 1..configs.columns {
                        idx = row * configs.columns + col;
                        data[idx] += data[idx - 1];
                    }
                }
            }
            // if ifd.interpret_as_u16()[0] == 3 {
            //     if configs.endian == Endianness::LittleEndian {
            //         if configs.data_type == DataType::F32 {
            //             let mut in_val: f32;
            //             let mut out_val: f32;
            //             let mut in_bytes: [u8; 4];
            //             let mut out_bytes = [0u8; 4];
            //             let mut idx: usize;
            //             for row in 0..configs.rows {
            //                 for col in 1..configs.columns {
            //                     idx = row * configs.columns + col;
            //                     in_val = data[idx] as f32;
            //                     in_bytes = in_val.to_le_bytes();
            //                     for b in 0..4 {
            //                         out_bytes[4-b-1] = in_bytes[b];
            //                     }
            //                     out_val = f32::from_le_bytes(out_bytes);
            //                     data[idx] = out_val as f64;
            //                 }
            //             }
            //         } else { // F64

            //         }
            //     } else {
            //         if configs.data_type == DataType::F32 {

            //         } else { // F64

            //         }
            //     }

            if ifd.interpret_as_u16()[0] == 3 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "The GeoTIFF reader does not currently support floating-point predictors (PREDICTOR=3).",
                ));
            }
        }
        _ => {} // do nothing,
    }

    // match geokeys_map.get(&1024) {
    //     Some(ifd) => geokeys.add_key_directory(&ifd.data),
    //     _ => return Err(Error::new(ErrorKind::InvalidData, "The TIFF file does not contain geokeys")),
    // };

    let mut map_sorter = vec![];
    for (key, _) in ifd_map.iter() {
        map_sorter.push(key);
    }
    map_sorter.sort();

    map_sorter.clear();
    for (key, _) in geokeys_map.iter() {
        map_sorter.push(key);
    }

    Ok(())
}

pub fn write_geotiff<'a>(r: &'a mut Raster) -> Result<(), Error> {
    // We'll need to look at the configurations to see if compression should be used
    let configs = whitebox_common::configs::get_configs()?;
    let use_compression = configs.compress_rasters;

    
    // get the ByteOrderWriter
    let f = File::create(r.file_name.clone())?;
    let mut writer = BufWriter::new(f);

    // let mut bow = ByteOrderWriter::<BufWriter<File>>::new(writer, r.configs.endian);

    // get the bytes per pixel
    let total_bytes_per_pixel = r.configs.data_type.get_data_size();
    if total_bytes_per_pixel == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Unknown data type: {:?}. Photomet interp: {:?}",
                r.configs.data_type, r.configs.photometric_interp
            ),
        ));
    }

    // is it a BigTiff?
    let is_big_tiff = if 8usize
        + (r.configs.rows * r.configs.columns) as usize * total_bytes_per_pixel
        >= 4_000_000_000
    {
        true
    } else {
        false
    };

    let header_size = if !is_big_tiff { 8u64 } else { 16u64 };

    // get the offset to the first ifd
    let mut ifd_start_needs_extra_byte = false;
    let mut ifd_start = if !use_compression {
        let mut val = header_size
            + (r.configs.rows * r.configs.columns) as u64 * total_bytes_per_pixel as u64;
        if val % 2 == 1 {
            val += 1;
            ifd_start_needs_extra_byte = true;
        }
        val
    } else {
        0u64
    };

    //////////////////////
    // Write the header //
    //////////////////////
    if r.configs.endian == Endianness::LittleEndian {
        write_bytes(&mut writer, "II".as_bytes()).expect("Error writing byte data.");
    } else {
        write_bytes(&mut writer, "MM".as_bytes()).expect("Error writing byte data.");
    }

    if !is_big_tiff {
        // magic number
        write_u16(&mut writer, r.configs.endian, 42u16).expect("Error writing byte data.");
        // offset to first IFD
        write_u32(&mut writer, r.configs.endian, ifd_start as u32)?;
    } else {
        // magic number
        write_u16(&mut writer, r.configs.endian, 43u16).expect("Error writing byte data.");
        // Bytesize of offsets
        write_u16(&mut writer, r.configs.endian, 8u16).expect("Error writing byte data.");

        write_u16(&mut writer, r.configs.endian, 0u16).expect("Error writing byte data."); // Always 0

        // offset to first IFD
        write_u64(&mut writer, r.configs.endian, ifd_start).expect("Error writing byte data.");
    }

    // At the moment, categorical and paletted output is not supported.
    if r.configs.photometric_interp == PhotometricInterpretation::Categorical
        || r.configs.photometric_interp == PhotometricInterpretation::Paletted
    {
        r.configs.photometric_interp = PhotometricInterpretation::Continuous;
    }

    //////////////////////////
    // Write the image data //
    //////////////////////////
    let mut strip_offsets = vec![];
    let mut strip_byte_counts = vec![];
    let mut current_offset = header_size;
    if use_compression {
        // DEFLATE is the only supported compression method at present

        // let mut current_offset = header_size;
        let mut row_length_in_bytes: u64;
        match r.configs.photometric_interp {
            PhotometricInterpretation::Continuous
            | PhotometricInterpretation::Categorical
            | PhotometricInterpretation::Boolean => match r.configs.data_type {
                DataType::F64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 8);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_f64::<LittleEndian>(r.data[i] as f64)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_f64::<BigEndian>(r.data[i] as f64)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::F32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 4);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_f32::<LittleEndian>(r.data[i] as f32)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_f32::<BigEndian>(r.data[i] as f32)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::U64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 8);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u64::<LittleEndian>(r.data[i] as u64)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u64::<BigEndian>(r.data[i] as u64)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::U32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 4);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u32::<LittleEndian>(r.data[i] as u32)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u32::<BigEndian>(r.data[i] as u32)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::U16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 2);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u16::<LittleEndian>(r.data[i] as u16)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u16::<BigEndian>(r.data[i] as u16)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::U8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u8(r.data[i] as u8)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_u8(r.data[i] as u8)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::I64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 8);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i64::<LittleEndian>(r.data[i] as i64)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i64::<BigEndian>(r.data[i] as i64)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::I32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 4);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i32::<LittleEndian>(r.data[i] as i32)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i32::<BigEndian>(r.data[i] as i32)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::I16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 2);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i16::<LittleEndian>(r.data[i] as i16)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i16::<BigEndian>(r.data[i] as i16)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::I8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i8(r.data[i] as i8)
                                    .expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_i8(r.data[i] as i8)
                                    .expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec_zlib(&data, 6);
                        write_bytes(&mut writer, &compressed)
                            .expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value).
                            write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Unknown data type: {:?}. Photomet interp: {:?}",
                            r.configs.data_type, r.configs.photometric_interp
                        ),
                    ));
                }
            },
            PhotometricInterpretation::RGB => {
                match r.configs.data_type {
                    DataType::RGB24 => {
                        // let mut bytes: [u8; 3] = [0u8; 3];
                        let mut i: usize;
                        let mut val: u32;
                        for row in 0..r.configs.rows {
                            let mut data = Vec::with_capacity(r.configs.columns * 3);
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                val = r.data[i] as u32;
                                data.write_u8((val & 0xFF) as u8)
                                    .expect("Error writing byte data."); // red

                                data.write_u8(((val >> 8u32) & 0xFF) as u8)
                                    .expect("Error writing byte data."); // green

                                data.write_u8(((val >> 16u32) & 0xFF) as u8)
                                    .expect("Error writing byte data."); // blue
                            }
                            // compress the data vec
                            let compressed = compress_to_vec_zlib(&data, 6);
                            write_bytes(&mut writer, &compressed)
                                .expect("Error writing byte data to file.");
                            row_length_in_bytes = compressed.len() as u64;
                            strip_byte_counts.push(row_length_in_bytes);
                            strip_offsets.push(current_offset);
                            current_offset += row_length_in_bytes;
                            if row_length_in_bytes % 2 != 0 {
                                // This is just because the data must start on a word (i.e. an even value).
                                write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                                current_offset += 1;
                            }
                        }
                    }
                    DataType::RGBA32 | DataType::U32 => {
                        let mut i: usize;
                        // let mut bytes: [u8; 4] = [0u8; 4];
                        let mut val: u32;
                        for row in 0..r.configs.rows {
                            let mut data = Vec::with_capacity(r.configs.columns * 4);
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                val = r.data[i] as u32;
                                data.write_u8((val & 0xFF) as u8)
                                    .expect("Error writing byte data."); // red

                                data.write_u8(((val >> 8u32) & 0xFF) as u8)
                                    .expect("Error writing byte data."); // green

                                data.write_u8(((val >> 16u32) & 0xFF) as u8)
                                    .expect("Error writing byte data."); // blue

                                data.write_u8(((val >> 24u32) & 0xFF) as u8)
                                    .expect("Error writing byte data."); // a
                            }
                            // for col in 0..r.configs.columns {
                            //     i = row * r.configs.columns + col;
                            //     val = r.data[i] as u32;
                            //     bytes[2] = ((val >> 16u32) & 0xFF) as u8; // blue
                            //     bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                            //     bytes[0] = (val & 0xFF) as u8; // red
                            //     bytes[3] = ((val >> 24u32) & 0xFF) as u8; // a
                            //     write_bytes(&mut writer, &bytes)?;
                            // }

                            // compress the data vec
                            let compressed = compress_to_vec_zlib(&data, 6);
                            write_bytes(&mut writer, &compressed)
                                .expect("Error writing byte data to file.");
                            row_length_in_bytes = compressed.len() as u64;
                            strip_byte_counts.push(row_length_in_bytes);
                            strip_offsets.push(current_offset);
                            current_offset += row_length_in_bytes;
                            if row_length_in_bytes % 2 != 0 {
                                // This is just because the data must start on a word (i.e. an even value).
                                write_u8(&mut writer, 0u8).expect("Error writing data to file.");
                                current_offset += 1;
                            }
                        }
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Unknown data type: {:?}. Photomet interp: {:?}",
                                r.configs.data_type, r.configs.photometric_interp
                            ),
                        ));
                    }
                }
            }
            PhotometricInterpretation::Paletted => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Paletted GeoTIFFs are currently unsupported for writing.",
                ));
            }
            PhotometricInterpretation::Unknown => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error while writing GeoTIFF file.",
                ));
            }
        }
    } else {
        match r.configs.photometric_interp {
            PhotometricInterpretation::Continuous
            | PhotometricInterpretation::Categorical
            | PhotometricInterpretation::Boolean => match r.configs.data_type {
                DataType::F64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_f64(&mut writer, r.configs.endian, r.data[i])?;
                        }
                    }
                }
                DataType::F32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_f32(&mut writer, r.configs.endian, r.data[i] as f32)?;
                        }
                    }
                }
                DataType::U64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_u64(&mut writer, r.configs.endian, r.data[i] as u64)?;
                        }
                    }
                }
                DataType::U32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_u32(&mut writer, r.configs.endian, r.data[i] as u32)?;
                        }
                    }
                }
                DataType::U16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_u16(&mut writer, r.configs.endian, r.data[i] as u16)?;
                        }
                    }
                }
                DataType::U8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_u8(&mut writer, r.data[i] as u8)?;
                        }
                    }
                }
                DataType::I64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_i64(&mut writer, r.configs.endian, r.data[i] as i64)?;
                        }
                    }
                }
                DataType::I32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_i32(&mut writer, r.configs.endian, r.data[i] as i32)?;
                        }
                    }
                }
                DataType::I16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_i16(&mut writer, r.configs.endian, r.data[i] as i16)?;
                        }
                    }
                }
                DataType::I8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            write_i8(&mut writer, r.data[i] as i8)?;
                        }
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Unknown data type: {:?}. Photomet interp: {:?}",
                            r.configs.data_type, r.configs.photometric_interp
                        ),
                    ));
                }
            },
            PhotometricInterpretation::RGB => {
                match r.configs.data_type {
                    DataType::RGB24 => {
                        let mut bytes: [u8; 3] = [0u8; 3];
                        let mut i: usize;
                        for row in 0..r.configs.rows {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                let val = r.data[i] as u32;
                                bytes[2] = ((val >> 16u32) & 0xFF) as u8; // blue
                                bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                                bytes[0] = (val & 0xFF) as u8; // red
                                write_bytes(&mut writer, &bytes)?;
                            }
                        }
                    }
                    DataType::RGBA32 | DataType::U32 => {
                        let mut i: usize;
                        let mut bytes: [u8; 4] = [0u8; 4];
                        for row in 0..r.configs.rows {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                let val = r.data[i] as u32;
                                bytes[2] = ((val >> 16u32) & 0xFF) as u8; // blue
                                bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                                bytes[0] = (val & 0xFF) as u8; // red
                                bytes[3] = ((val >> 24u32) & 0xFF) as u8; // a
                                write_bytes(&mut writer, &bytes)?;
                            }
                        }
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Unknown data type: {:?}. Photomet interp: {:?}",
                                r.configs.data_type, r.configs.photometric_interp
                            ),
                        ));
                    }
                }
            }
            PhotometricInterpretation::Paletted => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Paletted GeoTIFFs are currently unsupported for writing.",
                ));
            }
            PhotometricInterpretation::Unknown => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error while writing GeoTIFF file.",
                ));
            }
        }
    }

    if use_compression {
        ifd_start = current_offset; // header_size + strip_byte_counts.iter().sum();
        if ifd_start % 2 == 1 {
            ifd_start += 1;
            ifd_start_needs_extra_byte = true;
        }
        if !is_big_tiff {
            let _ = writer.seek(SeekFrom::Start(4));
            write_u32(&mut writer, r.configs.endian, ifd_start as u32)?;
        } else {
            let _ = writer.seek(SeekFrom::Start(8));
            write_u64(&mut writer, r.configs.endian, ifd_start)?;
        }
        let _ = writer.seek(SeekFrom::End(0));
    }

    // This is just because the IFD must start on a word (i.e. an even value). If the data are
    // single bytes, then this may not be the case.
    if ifd_start_needs_extra_byte {
        write_u8(&mut writer, 0u8).expect("Error writing byte data.");
    }

    ////////////////////////////
    // Create the IFD entries //
    ////////////////////////////

    /*
        Required Fields for Bilevel Images
        - ImageWidth
        - ImageLength
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
    */

    /*
        Required Fields for Grayscale Images
        - ImageWidth
        - ImageLength
        - BitsPerSample
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
    */

    /*
        Required Fields for Palette Colour Images
        - ImageWidth
        - ImageLength
        - BitsPerSample
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
        - ColorMap
    */

    /*
        Required Fields for RGB Images
        - ImageWidth
        - ImageLength
        - BitsPerSample
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - SamplesPerPixel
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
    */

    let mut ifd_entries: Vec<Entry> = vec![];
    // let mut larger_values_data: Vec<u8> = vec![];
    let mut larger_values_data = ByteOrderWriter::<Vec<u8>>::new(vec![], r.configs.endian);

    /*
    Classic TIFF IFD entries

    Bytes 0-1 The Tag that identifies the field.
    Bytes 2-3 The field Type.
    Bytes 4-7 The number of values, Count of the indicated Type.
    Bytes 8-11 The Value Offset, the file offset (in bytes) of the Value for the field.
    The Value is expected to begin on a word boundary; the corresponding
    Value Offset will thus be an even number. This file offset may
    point anywhere in the file, even after the image data.

    To save time and space the Value Offset contains the Value instead of pointing to
    the Value if and only if the Value fits into 4 bytes. If the Value is shorter than 4
    bytes, it is left-justified within the 4-byte Value Offset, i.e., stored in the lowernumbered
    bytes. Whether the Value fits within 4 bytes is determined by the Type
    and Count of the field.
    */

    // ImageWidth tag (256)
    ifd_entries.push(Entry::new(
        TAG_IMAGEWIDTH,
        DT_LONG,
        1u64,
        r.configs.columns as u64,
    ));

    // ImageLength tag (257)
    ifd_entries.push(Entry::new(
        TAG_IMAGELENGTH,
        DT_LONG,
        1u64,
        r.configs.rows as u64,
    ));

    let bits_per_sample = match r.configs.data_type {
        DataType::I8 | DataType::U8 => 8u16,
        DataType::I16 | DataType::U16 => 16u16,
        DataType::I32 | DataType::U32 | DataType::F32 => 32u16,
        DataType::I64 | DataType::U64 | DataType::F64 => 64u16,
        DataType::RGB24 => 8u16,
        DataType::RGBA32 => 8u16,
        DataType::RGB48 => 16u16,
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
        }
    };

    let samples_per_pixel = match r.configs.data_type {
        DataType::I8 | DataType::U8 => 1u16,
        DataType::I16 | DataType::U16 => 1u16,
        DataType::I32 | DataType::U32 | DataType::F32 => 1u16,
        DataType::I64 | DataType::U64 | DataType::F64 => 1u16,
        DataType::RGB24 => 3u16,
        DataType::RGBA32 => 4u16,
        DataType::RGB48 => 3u16,
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
        }
    };

    // BitsPerSample tag (258)
    if r.configs.photometric_interp != PhotometricInterpretation::Boolean {
        if samples_per_pixel == 1 {
            ifd_entries.push(Entry::new(
                TAG_BITSPERSAMPLE,
                DT_SHORT,
                samples_per_pixel as u64,
                bits_per_sample as u64,
            ));
        } else {
            ifd_entries.push(Entry::new(
                TAG_BITSPERSAMPLE,
                DT_SHORT,
                samples_per_pixel as u64,
                larger_values_data.len() as u64,
            ));
            for _ in 0..samples_per_pixel {
                larger_values_data.write_u16(bits_per_sample)?;
            }
        }
    }

    // Compression tag (259)
    if use_compression {
        ifd_entries.push(Entry::new(
            TAG_COMPRESSION,
            DT_SHORT,
            1u64,
            COMPRESS_DEFLATE as u64,
        ));
    } else {
        ifd_entries.push(Entry::new(
            TAG_COMPRESSION,
            DT_SHORT,
            1u64,
            COMPRESS_NONE as u64,
        ));
    }

    // PhotometricInterpretation tag (262)
    let pi = match r.configs.photometric_interp {
        PhotometricInterpretation::Continuous => PI_BLACKISZERO,
        PhotometricInterpretation::Categorical | PhotometricInterpretation::Paletted => PI_PALETTED,
        PhotometricInterpretation::Boolean => PI_BLACKISZERO,
        PhotometricInterpretation::RGB => PI_RGB,
        PhotometricInterpretation::Unknown => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Error while writing GeoTIFF file. Unknown Photometric Interpretation.",
            ));
        }
    };
    ifd_entries.push(Entry::new(
        TAG_PHOTOMETRICINTERPRETATION,
        DT_SHORT,
        1u64,
        pi as u64,
    ));

    // StripOffsets tag (273)
    if !is_big_tiff {
        ifd_entries.push(Entry::new(
            TAG_STRIPOFFSETS,
            DT_LONG,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        if use_compression {
            for val in strip_offsets {
                larger_values_data
                    .write_u32(val as u32)
                    .expect("Error writing the TIFF strip offsets tag");
            }
        } else {
            let row_length_in_bytes: u32 = r.configs.columns as u32 * total_bytes_per_pixel as u32;
            for i in 0..r.configs.rows as u32 {
                larger_values_data
                    .write_u32(header_size as u32 + row_length_in_bytes * i)
                    .expect("Error writing the TIFF strip offsets tag");
            }
        }
    } else {
        ifd_entries.push(Entry::new(
            TAG_STRIPOFFSETS,
            DT_TIFF_LONG8,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        if use_compression {
            for val in strip_offsets {
                larger_values_data
                    .write_u64(val)
                    .expect("Error writing the TIFF strip offsets tag");
            }
        } else {
            let row_length_in_bytes: u64 = r.configs.columns as u64 * total_bytes_per_pixel as u64;
            for i in 0..r.configs.rows as u64 {
                larger_values_data
                    .write_u64(header_size + row_length_in_bytes * i)
                    .expect("Error writing the TIFF strip offsets");
            }
        }
    }
    // if !is_big_tiff {
    //     ifd_entries.push(Entry::new(
    //         TAG_STRIPOFFSETS,
    //         DT_LONG,
    //         r.configs.rows as u64,
    //         larger_values_data.len() as u64,
    //     ));
    //     let row_length_in_bytes: u32 = r.configs.columns as u32 * total_bytes_per_pixel as u32;
    //     for i in 0..r.configs.rows as u32 {
    //         larger_values_data.write_u32(8u32 + row_length_in_bytes * i)?;
    //     }
    // } else {
    //     ifd_entries.push(Entry::new(
    //         TAG_STRIPOFFSETS,
    //         DT_TIFF_LONG8,
    //         r.configs.rows as u64,
    //         larger_values_data.len() as u64,
    //     ));
    //     let row_length_in_bytes: u64 = r.configs.columns as u64 * total_bytes_per_pixel as u64;
    //     for i in 0..r.configs.rows as u64 {
    //         larger_values_data.write_u64(8u64 + row_length_in_bytes * i)?;
    //     }
    // }

    // SamplesPerPixel tag (277)
    ifd_entries.push(Entry::new(
        TAG_SAMPLESPERPIXEL,
        DT_SHORT,
        1u64,
        samples_per_pixel as u64,
    ));

    // RowsPerStrip tag (278)
    ifd_entries.push(Entry::new(TAG_ROWSPERSTRIP, DT_SHORT, 1u64, 1u64));

    // StripByteCounts tag (279)
    if !is_big_tiff {
        ifd_entries.push(Entry::new(
            TAG_STRIPBYTECOUNTS,
            DT_LONG,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        let total_bytes_per_pixel = match r.configs.data_type {
            DataType::I8 | DataType::U8 => 1u32,
            DataType::I16 | DataType::U16 => 2u32,
            DataType::I32 | DataType::U32 | DataType::F32 => 4u32,
            DataType::I64 | DataType::U64 | DataType::F64 => 8u32,
            DataType::RGB24 => 3u32,
            DataType::RGBA32 => 4u32,
            DataType::RGB48 => 6u32,
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
            }
        };
        if use_compression {
            for val in strip_byte_counts {
                larger_values_data
                    .write_u32(val as u32)
                    .expect("Error writing the TIFF strip byte counts tag");
            }
        } else {
            let row_length_in_bytes: u32 = r.configs.columns as u32 * total_bytes_per_pixel;
            for _ in 0..r.configs.rows as u32 {
                larger_values_data
                    .write_u32(row_length_in_bytes)
                    .expect("Error writing the TIFF strip byte counts tag");
            }
        }
    } else {
        ifd_entries.push(Entry::new(
            TAG_STRIPBYTECOUNTS,
            DT_TIFF_LONG8,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        let total_bytes_per_pixel = match r.configs.data_type {
            DataType::I8 | DataType::U8 => 1u64,
            DataType::I16 | DataType::U16 => 2u64,
            DataType::I32 | DataType::U32 | DataType::F32 => 4u64,
            DataType::I64 | DataType::U64 | DataType::F64 => 8u64,
            DataType::RGB24 => 3u64,
            DataType::RGBA32 => 4u64,
            DataType::RGB48 => 6u64,
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
            }
        };
        if use_compression {
            for val in strip_byte_counts {
                larger_values_data
                    .write_u64(val)
                    .expect("Error writing the TIFF strip byte counts tag");
            }
        } else {
            let row_length_in_bytes: u64 = r.configs.columns as u64 * total_bytes_per_pixel;
            for _ in 0..r.configs.rows as u32 {
                larger_values_data
                    .write_u64(row_length_in_bytes)
                    .expect("Error writing the TIFF strip byte counts tag");
            }
        }
    }
    /*
    if !is_big_tiff {
        ifd_entries.push(Entry::new(
            TAG_STRIPBYTECOUNTS,
            DT_LONG,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        let total_bytes_per_pixel = match r.configs.data_type {
            DataType::I8 | DataType::U8 => 1u32,
            DataType::I16 | DataType::U16 => 2u32,
            DataType::I32 | DataType::U32 | DataType::F32 => 4u32,
            DataType::I64 | DataType::U64 | DataType::F64 => 8u32,
            DataType::RGB24 => 3u32,
            DataType::RGBA32 => 4u32,
            DataType::RGB48 => 6u32,
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
            }
        };
        let row_length_in_bytes: u32 = r.configs.columns as u32 * total_bytes_per_pixel;
        for _ in 0..r.configs.rows as u32 {
            larger_values_data.write_u32(row_length_in_bytes)?;
        }
    } else {
        ifd_entries.push(Entry::new(
            TAG_STRIPBYTECOUNTS,
            DT_TIFF_LONG8,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        let total_bytes_per_pixel = match r.configs.data_type {
            DataType::I8 | DataType::U8 => 1u64,
            DataType::I16 | DataType::U16 => 2u64,
            DataType::I32 | DataType::U32 | DataType::F32 => 4u64,
            DataType::I64 | DataType::U64 | DataType::F64 => 8u64,
            DataType::RGB24 => 3u64,
            DataType::RGBA32 => 4u64,
            DataType::RGB48 => 6u64,
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
            }
        };
        let row_length_in_bytes: u64 = r.configs.columns as u64 * total_bytes_per_pixel;
        for _ in 0..r.configs.rows as u32 {
            larger_values_data.write_u64(row_length_in_bytes)?;
        }
    }
    */

    // There is currently no support for storing the image resolution, so give a bogus value of 72x72 dpi.
    // XResolution tag (282)
    ifd_entries.push(Entry::new(
        TAG_XRESOLUTION,
        DT_RATIONAL,
        1u64,
        larger_values_data.len() as u64,
    ));
    larger_values_data.write_u32(72u32)?;
    larger_values_data.write_u32(1u32)?;

    // YResolution tag (283)
    ifd_entries.push(Entry::new(
        TAG_YRESOLUTION,
        DT_RATIONAL,
        1u64,
        larger_values_data.len() as u64,
    ));
    larger_values_data.write_u32(72u32)?;
    larger_values_data.write_u32(1u32)?;

    // ResolutionUnit tag (296)
    ifd_entries.push(Entry::new(TAG_RESOLUTIONUNIT, DT_SHORT, 1u64, 2u64));

    // Software tag (305)
    let software = "WhiteboxTools".to_owned();
    let mut soft_bytes = software.into_bytes();
    soft_bytes.push(0);
    ifd_entries.push(Entry::new(
        TAG_SOFTWARE,
        DT_ASCII,
        soft_bytes.len() as u64,
        larger_values_data.len() as u64,
    ));
    larger_values_data.write_bytes(&soft_bytes)?;

    if samples_per_pixel == 4 {
        // ExtraSamples tag (338)
        ifd_entries.push(Entry::new(TAG_EXTRASAMPLES, DT_SHORT, 1u64, 2u64));
    }

    // SampleFormat tag (339)
    let samples_format = match r.configs.data_type {
        DataType::U8 | DataType::U16 | DataType::U32 | DataType::U64 => 1u16,
        DataType::I8 | DataType::I16 | DataType::I32 | DataType::I64 => 2u16,
        DataType::F32 | DataType::F64 => 3u16,
        DataType::RGB24 | DataType::RGBA32 | DataType::RGB48 => 1u16,
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
        }
    };
    if samples_per_pixel == 1 {
        ifd_entries.push(Entry::new(
            TAG_SAMPLEFORMAT,
            DT_SHORT,
            samples_per_pixel as u64,
            samples_format as u64,
        ));
    } else {
        ifd_entries.push(Entry::new(
            TAG_SAMPLEFORMAT,
            DT_SHORT,
            samples_per_pixel as u64,
            larger_values_data.len() as u64,
        ));
        for _ in 0..samples_per_pixel {
            larger_values_data.write_u16(samples_format)?;
        }
    }

    // ModelPixelScaleTag tag (33550)
    if r.configs.model_pixel_scale[0] == 0f64
        && r.configs.model_tiepoint.is_empty()
        && r.configs.model_transformation[0] == 0f64
    {
        ifd_entries.push(Entry::new(
            TAG_MODELPIXELSCALETAG,
            DT_DOUBLE,
            3u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_f64(r.configs.resolution_x)?;
        larger_values_data.write_f64(r.configs.resolution_y)?;
        larger_values_data.write_f64(0f64)?;
    } else if r.configs.model_pixel_scale[0] != 0f64 {
        ifd_entries.push(Entry::new(
            TAG_MODELPIXELSCALETAG,
            DT_DOUBLE,
            3u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_f64(r.configs.model_pixel_scale[0])?;
        larger_values_data.write_f64(r.configs.model_pixel_scale[1])?;
        larger_values_data.write_f64(r.configs.model_pixel_scale[2])?;
    }

    if r.configs.model_tiepoint.is_empty() && r.configs.model_transformation[0] == 0f64 {
        // ModelTiepointTag tag (33922)
        ifd_entries.push(Entry::new(
            TAG_MODELTIEPOINTTAG,
            DT_DOUBLE,
            6u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_f64(0f64)?; // I
        larger_values_data.write_f64(0f64)?; // J
        larger_values_data.write_f64(0f64)?; // K
        larger_values_data.write_f64(r.configs.west)?; // X
        larger_values_data.write_f64(r.configs.north)?; // Y
        larger_values_data.write_f64(0f64)?; // Z
    } else if !r.configs.model_tiepoint.is_empty() {
        // ModelTiepointTag tag (33922)
        ifd_entries.push(Entry::new(
            TAG_MODELTIEPOINTTAG,
            DT_DOUBLE,
            r.configs.model_tiepoint.len() as u64,
            larger_values_data.len() as u64,
        ));
        for i in 0..r.configs.model_tiepoint.len() {
            larger_values_data.write_f64(r.configs.model_tiepoint[i])?;
        }
    }

    if r.configs.model_transformation[0] != 0f64 {
        // ModelTransformationTag tag (33920)
        ifd_entries.push(Entry::new(
            TAG_MODELTRANSFORMATIONTAG,
            DT_DOUBLE,
            16u64,
            larger_values_data.len() as u64,
        ));
        for i in 0..16 {
            larger_values_data.write_f64(r.configs.model_transformation[i])?;
        }
    }

    // TAG_GDAL_NODATA tag (42113)
    let nodata_str = format!("{}", r.configs.nodata);
    let mut nodata_bytes = nodata_str.into_bytes();
    if !is_big_tiff {
        // we buffer this string with spaces to ensure that it is
        // long enough to be printed to larger_values_data.
        if nodata_bytes.len() < 4 {
            for _ in 0..(4 - nodata_bytes.len()) {
                nodata_bytes.push(32);
            }
        }
        if nodata_bytes.len() % 2 == 0 {
            nodata_bytes.push(32);
        }
        nodata_bytes.push(0);
        ifd_entries.push(Entry::new(
            TAG_GDAL_NODATA,
            DT_ASCII,
            nodata_bytes.len() as u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_bytes(&nodata_bytes)?;
    } else {
        // we buffer this string with spaces to ensure that it is
        // long enough to be printed to larger_values_data.
        if nodata_bytes.len() < 8 {
            for _ in 0..(8 - nodata_bytes.len()) {
                nodata_bytes.push(32);
            }
        }
        if nodata_bytes.len() % 2 == 0 {
            nodata_bytes.push(32);
        }
        nodata_bytes.push(0);
        ifd_entries.push(Entry::new(
            TAG_GDAL_NODATA,
            DT_ASCII,
            nodata_bytes.len() as u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_bytes(&nodata_bytes)?;
    }

    let kw_map = get_keyword_map();
    let geographic_type_map = match kw_map.get(&2048u16) {
        Some(map) => map,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Error generating geographic type map.",
            ))
        }
    };
    let projected_cs_type_map = match kw_map.get(&3072u16) {
        Some(map) => map,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Error generating projected coordinate system type map.",
            ))
        }
    };

    //let key_map = get_keys_map();
    let mut gk_entries: Vec<GeoKeyEntry> = vec![];
    let mut ascii_params = String::new(); //: Vec<u8> = vec![];
    let double_params: Vec<f64> = vec![];
    if geographic_type_map.contains_key(&r.configs.epsg_code) {
        // tGTModelTypeGeoKey (1024)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTMODELTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: 2u16,
        });

        // GTRasterTypeGeoKey (1025)
        if r.configs.pixel_is_area {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 1u16,
            });
        } else {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 2u16,
            });
        }

        // tGTCitationGeoKey (1026)
        let mut v = String::from(
            geographic_type_map
                .get(&r.configs.epsg_code)
                .unwrap()
                .clone(),
        );
        v.push_str("|");
        v = v.replace("_", " ");
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTCITATIONGEOKEY,
            location: 34737u16,
            count: v.len() as u16,
            value_offset: ascii_params.len() as u16,
        });
        ascii_params.push_str(&v);

        // tGeographicTypeGeoKey (2048)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GEOGRAPHICTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: r.configs.epsg_code,
        });

        if r.configs.z_units.to_lowercase() != "not specified" {
            // VerticalUnitsGeoKey (4099)
            let units = r.configs.z_units.to_lowercase();
            if units.contains("met") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9001u16,
                });
            } else if units.contains("ft") | units.contains("feet") | units.contains("foot") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9002u16,
                });
            }
        }
    } else if projected_cs_type_map.contains_key(&r.configs.epsg_code) {
        // tGTModelTypeGeoKey (1024)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTMODELTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: 1u16,
        });

        // GTRasterTypeGeoKey (1025)
        if r.configs.pixel_is_area {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 1u16,
            });
        } else {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 2u16,
            });
        }

        // tProjectedCSTypeGeoKey (3072)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_PROJECTEDCSTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: r.configs.epsg_code,
        });

        // PCSCitationGeoKey (3073)
        let mut v = String::from(
            projected_cs_type_map
                .get(&r.configs.epsg_code)
                .unwrap()
                .clone(),
        );
        v.push_str("|");
        v = v.replace("_", " ");
        gk_entries.push(GeoKeyEntry {
            tag: 3073u16,
            location: 34737u16,
            count: v.len() as u16,
            value_offset: ascii_params.len() as u16,
        });
        ascii_params.push_str(&v);

        if r.configs.xy_units.to_lowercase() != "not specified" {
            // ProjLinearUnitsGeoKey (3076)
            let units = r.configs.xy_units.to_lowercase();
            if units.contains("met") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_PROJLINEARUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9001u16,
                });
            } else if units.contains("ft") | units.contains("feet") | units.contains("foot") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_PROJLINEARUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9002u16,
                });
            }
        }

        if r.configs.z_units.to_lowercase() != "not specified" {
            // VerticalUnitsGeoKey (4099)
            let units = r.configs.z_units.to_lowercase();
            if units.contains("met") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9001u16,
                });
            } else if units.contains("ft") | units.contains("feet") | units.contains("foot") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9002u16,
                });
            }
        }
    } else {
        // we don't know much about the coordinate system used.

        // tGTModelTypeGeoKey (1024)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTMODELTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: 0u16,
        });

        // GTRasterTypeGeoKey (1025)
        if r.configs.pixel_is_area {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 1u16,
            });
        } else {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 2u16,
            });
        }
    }

    if r.configs.geo_key_directory.is_empty() {
        // create the GeoKeyDirectoryTag tag (34735)
        ifd_entries.push(Entry::new(
            TAG_GEOKEYDIRECTORYTAG,
            DT_SHORT,
            (4 + gk_entries.len() * 4) as u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_u16(1u16)?; // KeyDirectoryVersion
        larger_values_data.write_u16(1u16)?; // KeyRevision
        larger_values_data.write_u16(0u16)?; // MinorRevision
        larger_values_data.write_u16(gk_entries.len() as u16)?; // NumberOfKeys

        for entry in gk_entries {
            larger_values_data.write_u16(entry.tag)?; // KeyID
            larger_values_data.write_u16(entry.location)?; // TIFFTagLocation
            larger_values_data.write_u16(entry.count)?; // Count
            larger_values_data.write_u16(entry.value_offset)?; // Value_Offset
        }

        if double_params.len() > 0 {
            // create the GeoDoubleParamsTag tag (34736)
            ifd_entries.push(Entry::new(
                TAG_GEODOUBLEPARAMSTAG,
                DT_DOUBLE,
                double_params.len() as u64,
                larger_values_data.len() as u64,
            ));
            for double_val in double_params {
                larger_values_data.write_f64(double_val)?;
            }
        }

        if ascii_params.len() > 0 {
            // create the GeoAsciiParamsTag tag (34737)
            let mut ascii_params_bytes = ascii_params.into_bytes();
            ascii_params_bytes.push(0);
            ifd_entries.push(Entry::new(
                TAG_GEOASCIIPARAMSTAG,
                DT_ASCII,
                ascii_params_bytes.len() as u64,
                larger_values_data.len() as u64,
            ));
            if ascii_params_bytes.len() % 2 == 1 {
                // it has to end on a word so that the next value starts on a word
                ascii_params_bytes.push(0);
            }
            larger_values_data.write_bytes(&ascii_params_bytes)?;
        }
    } else {
        // let num_keys = (r.configs.geo_key_directory.len() - 4) / 4;
        // output the GeoKeyDirectoryTag tag (34735)
        ifd_entries.push(Entry::new(
            TAG_GEOKEYDIRECTORYTAG,
            DT_SHORT,
            r.configs.geo_key_directory.len() as u64,
            larger_values_data.len() as u64,
        ));
        for val in &r.configs.geo_key_directory {
            larger_values_data.write_u16(*val)?;
        }

        if r.configs.geo_double_params.len() > 0 {
            // create the GeoDoubleParamsTag tag (34736)
            ifd_entries.push(Entry::new(
                TAG_GEODOUBLEPARAMSTAG,
                DT_DOUBLE,
                r.configs.geo_double_params.len() as u64,
                larger_values_data.len() as u64,
            ));
            for double_val in &r.configs.geo_double_params {
                larger_values_data.write_f64(*double_val)?;
            }
        }

        if !r.configs.geo_ascii_params.is_empty() {
            // create the GeoAsciiParamsTag tag (34737)
            let mut ascii_params_bytes = r.configs.geo_ascii_params.clone().into_bytes();
            ascii_params_bytes.push(0);
            ifd_entries.push(Entry::new(
                TAG_GEOASCIIPARAMSTAG,
                DT_ASCII,
                ascii_params_bytes.len() as u64,
                larger_values_data.len() as u64,
            ));
            if ascii_params_bytes.len() % 2 == 1 {
                // it has to end on a word so that the next value starts on a word
                ascii_params_bytes.push(0);
            }
            larger_values_data.write_bytes(&ascii_params_bytes)?;
        }
    }

    ///////////////////
    // Write the IFD //
    ///////////////////

    // Number of Directory Entries.
    if !is_big_tiff {
        write_u16(&mut writer, r.configs.endian, ifd_entries.len() as u16)?;

        // Sort the IFD entries
        ifd_entries.sort_by(|a, b| a.tag.cmp(&b.tag));

        // Write the entries
        let ifd_length = 2u64 + ifd_entries.len() as u64 * 12u64 + 4u64;

        for ifde in ifd_entries {
            write_u16(&mut writer, r.configs.endian, ifde.tag)?; // Tag
            write_u16(&mut writer, r.configs.endian, ifde.ifd_type)?; // Field type
            write_u32(&mut writer, r.configs.endian, ifde.num_values as u32)?; // Num of values
            if ifde.ifd_type == DT_SHORT && ifde.num_values == 1 {
                // it's a value
                write_u16(&mut writer, r.configs.endian, ifde.offset as u16)?; // Value
                write_u16(&mut writer, r.configs.endian, 0u16)?; // Fill the remaining 2 right bytes of the u32
            } else if ifde.ifd_type == DT_LONG && ifde.num_values == 1 {
                // it's a value
                write_u32(&mut writer, r.configs.endian, ifde.offset as u32)?;
            } else if ifde.ifd_type == DT_SHORT && ifde.num_values == 2 {
                // I'm not really sure about this one. Two shorts will fit in the value_offset, but will they be interpreted correctly?
                write_u32(&mut writer, r.configs.endian, ifde.offset as u32)?; // Value
            } else {
                // it's an offset
                write_u32(
                    &mut writer,
                    r.configs.endian,
                    ifd_start as u32 + ifd_length as u32 + ifde.offset as u32,
                )?;
            }
        }

        // 4-byte offset of the next IFD; Note, only single image TIFFs are currently supported
        // and therefore, this will always be set to '0'.
        write_u32(&mut writer, r.configs.endian, 0u32)?;
    } else {
        write_u64(&mut writer, r.configs.endian, ifd_entries.len() as u64)?;

        // Sort the IFD entries
        ifd_entries.sort_by(|a, b| a.tag.cmp(&b.tag));

        // Write the entries
        let ifd_length = 8u64 + ifd_entries.len() as u64 * 20u64 + 8u64;

        for ifde in ifd_entries {
            write_u16(&mut writer, r.configs.endian, ifde.tag)?; // Tag
            write_u16(&mut writer, r.configs.endian, ifde.ifd_type)?; // Field type
            write_u64(&mut writer, r.configs.endian, ifde.num_values)?; // Num of values
            if ifde.ifd_type == DT_SHORT && ifde.num_values == 1 {
                // it's a value
                write_u16(&mut writer, r.configs.endian, ifde.offset as u16)?; // Value
                write_u16(&mut writer, r.configs.endian, 0u16)?; // Fill the remaining bytes of the u64
                write_u32(&mut writer, r.configs.endian, 0u32)?; // Fill the remaining bytes of the u64
            } else if ifde.ifd_type == DT_SHORT && ifde.num_values == 2 {
                // I'm not really sure about this one. Two shorts will fit in the value_offset, but will they be interpreted correctly?
                write_u32(&mut writer, r.configs.endian, ifde.offset as u32)?; // Value
                write_u32(&mut writer, r.configs.endian, 0u32)?; // Fill the remaining bytes of the u64
            } else if ifde.ifd_type == DT_LONG && ifde.num_values == 1 {
                // it's a value
                write_u32(&mut writer, r.configs.endian, ifde.offset as u32)?;
                write_u32(&mut writer, r.configs.endian, 0u32)?; // Fill the remaining bytes of the u64
            } else if (ifde.ifd_type == DT_LONG && ifde.num_values == 2)
                || (ifde.ifd_type == DT_TIFF_LONG8 && ifde.num_values == 1)
            {
                // it's a value
                write_u64(&mut writer, r.configs.endian, ifde.offset)?;
            } else {
                // it's an offset
                write_u64(
                    &mut writer,
                    r.configs.endian,
                    ifd_start + ifd_length + ifde.offset,
                )?;
            }
        }

        // 4-byte offset of the next IFD; Note, only single image TIFFs are currently supported
        // and therefore, this will always be set to '0'.
        write_u64(&mut writer, r.configs.endian, 0u64)?;
    }

    //////////////////////////////////
    // Write the larger_values_data //
    //////////////////////////////////
    write_bytes(&mut writer, larger_values_data.get_inner())?;

    Ok(())
}

/*
pub fn write_geotiff<'a>(r: &'a mut Raster) -> Result<(), Error> {
    // get the ByteOrderWriter
    let f = File::create(r.file_name.clone())?;
    let writer = BufWriter::new(f);
    let mut bow = ByteOrderWriter::<BufWriter<File>>::new(writer, r.configs.endian);

    // get the bytes per pixel
    let total_bytes_per_pixel = r.configs.data_type.get_data_size();
    if total_bytes_per_pixel == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Unknown data type: {:?}. Photomet interp: {:?}",
                r.configs.data_type, r.configs.photometric_interp
            ),
        ));
    }

    // is it a BigTiff?
    let is_big_tiff = if 8usize
        + (r.configs.rows * r.configs.columns) as usize * total_bytes_per_pixel
        >= 4_000_000_000
    {
        true
    } else {
        false
    };

    let header_size = if is_big_tiff {
        16u64
    } else {
        8u64
    };

    let ifd_start = 0u64; // only temporary because we don't know the length of the image data (if compression is used).
    // // get the offset to the first ifd
    // let mut ifd_start = if !is_big_tiff {
    //     (8usize + (r.configs.rows * r.configs.columns) as usize * total_bytes_per_pixel) as u64
    // // plus the 8-byte header
    // } else {
    //     (16usize + (r.configs.rows * r.configs.columns) as usize * total_bytes_per_pixel) as u64
    //     // plus the 16-byte header
    // };
    // let mut ifd_start_needs_extra_byte = false;
    // if ifd_start % 2 == 1 {
    //     ifd_start += 1;
    //     ifd_start_needs_extra_byte = true;
    // }

    //////////////////////
    // Write the header //
    //////////////////////

    if r.configs.endian == Endianness::LittleEndian {
        bow.write_bytes("II".as_bytes())?;
    } else {
        bow.write_bytes("MM".as_bytes())?;
    }

    if !is_big_tiff {
        // magic number
        bow.write_u16(42u16)?;
        // offset to first IFD
        bow.write_u32(ifd_start as u32)?;
    } else {
        // magic number
        bow.write_u16(43u16)?;
        // Bytesize of offsets
        bow.write_u16(8u16)?;

        bow.write_u16(0u16)?; // Always 0

        // offset to first IFD; This needs to be updated if compression is used.
        bow.write_u64(ifd_start)?;
    }

    // At the moment, categorical and paletted output is not supported.
    if r.configs.photometric_interp == PhotometricInterpretation::Categorical
        || r.configs.photometric_interp == PhotometricInterpretation::Paletted
    {
        r.configs.photometric_interp = PhotometricInterpretation::Continuous;
    }

    //////////////////////////
    // Write the image data //
    //////////////////////////

    let use_compression = true;
    let mut strip_offsets = vec![];
    let mut strip_byte_counts = vec![];
    if use_compression {
        // DEFLATE is the only supported compression method at present

        let mut current_offset = header_size;
        let mut row_length_in_bytes: u64;

        match r.configs.photometric_interp {
            PhotometricInterpretation::Continuous
            | PhotometricInterpretation::Categorical
            | PhotometricInterpretation::Boolean => match r.configs.data_type {
                DataType::F64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_f64(r.data[i])?;
                        }
                    }
                }
                DataType::F32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        let mut data = Vec::with_capacity(r.configs.columns * 4);
                        if r.configs.endian == Endianness::LittleEndian {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_f32::<LittleEndian>(r.data[i] as f32).expect("Error writing byte data.");
                            }
                        } else {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                data.write_f32::<BigEndian>(r.data[i] as f32).expect("Error writing byte data.");
                            }
                        }
                        // compress the data vec
                        let compressed = compress_to_vec(&data, 6);
                        bow.write_bytes(&compressed).expect("Error writing byte data to file.");
                        row_length_in_bytes = compressed.len() as u64;
                        strip_byte_counts.push(row_length_in_bytes);
                        strip_offsets.push(current_offset);
                        current_offset += row_length_in_bytes;
                        if row_length_in_bytes % 2 != 0 {
                            // This is just because the data must start on a word (i.e. an even value). If the data are
                            // single bytes, then this may not be the case.
                            bow.write_u8(0u8).expect("Error writing data to file.");
                            current_offset += 1;
                        }
                    }
                }
                DataType::U64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u64(r.data[i] as u64)?;
                        }
                    }
                }
                DataType::U32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u32(r.data[i] as u32)?;
                        }
                    }
                }
                DataType::U16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u16(r.data[i] as u16)?;
                        }
                    }
                }
                DataType::U8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u8(r.data[i] as u8)?;
                        }
                    }
                }
                DataType::I64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i64(r.data[i] as i64)?;
                        }
                    }
                }
                DataType::I32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i32(r.data[i] as i32)?;
                        }
                    }
                }
                DataType::I16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i16(r.data[i] as i16)?;
                        }
                    }
                }
                DataType::I8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i8(r.data[i] as i8)?;
                        }
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Unknown data type: {:?}. Photomet interp: {:?}",
                            r.configs.data_type, r.configs.photometric_interp
                        ),
                    ));
                }
            },
            PhotometricInterpretation::RGB => {
                match r.configs.data_type {
                    DataType::RGB24 => {
                        let mut bytes: [u8; 3] = [0u8; 3];
                        let mut i: usize;
                        for row in 0..r.configs.rows {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                let val = r.data[i] as u32;
                                bytes[2] = ((val >> 16u32) & 0xFF) as u8; // blue
                                bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                                bytes[0] = (val & 0xFF) as u8; // red
                                bow.write_bytes(&bytes).expect("Error writing bytes to file.");
                            }
                        }
                    }
                    DataType::RGBA32 | DataType::U32 => {
                        let mut i: usize;
                        let mut bytes: [u8; 4] = [0u8; 4];
                        for row in 0..r.configs.rows {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                let val = r.data[i] as u32;
                                bytes[2] = ((val >> 16u32) & 0xFF) as u8; // blue
                                bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                                bytes[0] = (val & 0xFF) as u8; // red
                                bytes[3] = ((val >> 24u32) & 0xFF) as u8; // a
                                bow.write_bytes(&bytes).expect("Error writing bytes to file.");
                            }
                        }
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Unknown data type: {:?}. Photomet interp: {:?}",
                                r.configs.data_type, r.configs.photometric_interp
                            ),
                        ));
                    }
                }
            }
            PhotometricInterpretation::Paletted => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Paletted GeoTIFFs are currently unsupported for writing.",
                ));
            }
            PhotometricInterpretation::Unknown => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error while writing GeoTIFF file.",
                ));
            }
        }
    } else {

        match r.configs.photometric_interp {
            PhotometricInterpretation::Continuous
            | PhotometricInterpretation::Categorical
            | PhotometricInterpretation::Boolean => match r.configs.data_type {
                DataType::F64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_f64(r.data[i]).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::F32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_f32(r.data[i] as f32).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::U64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u64(r.data[i] as u64).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::U32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u32(r.data[i] as u32).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::U16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u16(r.data[i] as u16).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::U8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_u8(r.data[i] as u8).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::I64 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i64(r.data[i] as i64).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::I32 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i32(r.data[i] as i32).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::I16 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i16(r.data[i] as i16).expect("Error writing byte data to file.");
                        }
                    }
                }
                DataType::I8 => {
                    let mut i: usize;
                    for row in 0..r.configs.rows {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            bow.write_i8(r.data[i] as i8).expect("Error writing byte data to file.");
                        }
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Unknown data type: {:?}. Photomet interp: {:?}",
                            r.configs.data_type, r.configs.photometric_interp
                        ),
                    ));
                }
            },
            PhotometricInterpretation::RGB => {
                match r.configs.data_type {
                    DataType::RGB24 => {
                        let mut bytes: [u8; 3] = [0u8; 3];
                        let mut i: usize;
                        for row in 0..r.configs.rows {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                let val = r.data[i] as u32;
                                bytes[2] = ((val >> 16u32) & 0xFF) as u8; // blue
                                bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                                bytes[0] = (val & 0xFF) as u8; // red
                                bow.write_bytes(&bytes).expect("Error writing byte data to file.");
                            }
                        }
                    }
                    DataType::RGBA32 | DataType::U32 => {
                        let mut i: usize;
                        let mut bytes: [u8; 4] = [0u8; 4];
                        for row in 0..r.configs.rows {
                            for col in 0..r.configs.columns {
                                i = row * r.configs.columns + col;
                                let val = r.data[i] as u32;
                                bytes[2] = ((val >> 16u32) & 0xFF) as u8; // blue
                                bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                                bytes[0] = (val & 0xFF) as u8; // red
                                bytes[3] = ((val >> 24u32) & 0xFF) as u8; // a
                                bow.write_bytes(&bytes).expect("Error writing byte data to file.");
                            }
                        }
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Unknown data type: {:?}. Photomet interp: {:?}",
                                r.configs.data_type, r.configs.photometric_interp
                            ),
                        ));
                    }
                }
            }
            PhotometricInterpretation::Paletted => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Paletted GeoTIFFs are currently unsupported for writing.",
                ));
            }
            PhotometricInterpretation::Unknown => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error while writing GeoTIFF file.",
                ));
            }
        }
    }

    // // This is just because the IFD must start on a word (i.e. an even value). If the data are
    // // single bytes, then this may not be the case.
    // if ifd_start_needs_extra_byte {
    //     bow.write_u8(0u8)?;
    // }

    let mut current_file_length = bow.len() as u64;
    if current_file_length % 2 != 0 {
        // This is just because the IFD must start on a word (i.e. an even value). If the data are
        // single bytes, then this may not be the case.
        bow.write_u8(0u8).expect("Error writing data to file.");
        current_file_length += 1;
    }

    // We need to update the offset to the IFD, now that we know the size of the image data,
    // taking into account compression.
    if !is_big_tiff {
        bow.seek_from_start(4u64);
        if r.configs.endian == Endianness::LittleEndian {
            bow.write_u32(current_file_length as u32)?;
        } else {
            bow.write_u32(current_file_length as u32)?;
        }
    } else {
        bow.seek_from_start(8u64);
        if r.configs.endian == Endianness::LittleEndian {
            bow.write_u64(current_file_length)?;
        } else {
            bow.write_u64(current_file_length)?;
        }
    }
    bow.seek_end(); // return the bow to the end of the file.

    // We need to update the offset to the IFD in the header now.


    ////////////////////////////
    // Create the IFD entries //
    ////////////////////////////

    /*
        Required Fields for Bilevel Images
        - ImageWidth
        - ImageLength
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
    */

    /*
        Required Fields for Grayscale Images
        - ImageWidth
        - ImageLength
        - BitsPerSample
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
    */

    /*
        Required Fields for Palette Colour Images
        - ImageWidth
        - ImageLength
        - BitsPerSample
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
        - ColorMap
    */

    /*
        Required Fields for RGB Images
        - ImageWidth
        - ImageLength
        - BitsPerSample
        - Compression
        - PhotometricInterpretation
        - StripOffsets
        - SamplesPerPixel
        - RowsPerStrip
        - StripByteCounts
        - XResolution
        - YResolution
        - ResolutionUnit
    */

    let mut ifd_entries: Vec<Entry> = vec![];
    // let mut larger_values_data: Vec<u8> = vec![];
    let mut larger_values_data = ByteOrderWriter::<std::io::Cursor<Vec<u8>>>::new(std::io::Cursor::new(vec![]), r.configs.endian);

    /*
    Classic TIFF IFD entries

    Bytes 0-1 The Tag that identifies the field.
    Bytes 2-3 The field Type.
    Bytes 4-7 The number of values, Count of the indicated Type.
    Bytes 8-11 The Value Offset, the file offset (in bytes) of the Value for the field.
    The Value is expected to begin on a word boundary; the corresponding
    Value Offset will thus be an even number. This file offset may
    point anywhere in the file, even after the image data.

    To save time and space the Value Offset contains the Value instead of pointing to
    the Value if and only if the Value fits into 4 bytes. If the Value is shorter than 4
    bytes, it is left-justified within the 4-byte Value Offset, i.e., stored in the lowernumbered
    bytes. Whether the Value fits within 4 bytes is determined by the Type
    and Count of the field.
    */

    // ImageWidth tag (256)
    ifd_entries.push(Entry::new(
        TAG_IMAGEWIDTH,
        DT_LONG,
        1u64,
        r.configs.columns as u64,
    ));

    // ImageLength tag (257)
    ifd_entries.push(Entry::new(
        TAG_IMAGELENGTH,
        DT_LONG,
        1u64,
        r.configs.rows as u64,
    ));

    let bits_per_sample = match r.configs.data_type {
        DataType::I8 | DataType::U8 => 8u16,
        DataType::I16 | DataType::U16 => 16u16,
        DataType::I32 | DataType::U32 | DataType::F32 => 32u16,
        DataType::I64 | DataType::U64 | DataType::F64 => 64u16,
        DataType::RGB24 => 8u16,
        DataType::RGBA32 => 8u16,
        DataType::RGB48 => 16u16,
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
        }
    };

    let samples_per_pixel = match r.configs.data_type {
        DataType::I8 | DataType::U8 => 1u16,
        DataType::I16 | DataType::U16 => 1u16,
        DataType::I32 | DataType::U32 | DataType::F32 => 1u16,
        DataType::I64 | DataType::U64 | DataType::F64 => 1u16,
        DataType::RGB24 => 3u16,
        DataType::RGBA32 => 4u16,
        DataType::RGB48 => 3u16,
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
        }
    };

    // BitsPerSample tag (258)
    if r.configs.photometric_interp != PhotometricInterpretation::Boolean {
        if samples_per_pixel == 1 {
            ifd_entries.push(Entry::new(
                TAG_BITSPERSAMPLE,
                DT_SHORT,
                samples_per_pixel as u64,
                bits_per_sample as u64,
            ));
        } else {
            ifd_entries.push(Entry::new(
                TAG_BITSPERSAMPLE,
                DT_SHORT,
                samples_per_pixel as u64,
                larger_values_data.len() as u64,
            ));
            for _ in 0..samples_per_pixel {
                larger_values_data.write_u16(bits_per_sample)?;
            }
        }
    }

    // Compression tag (259)
    if use_compression {
        ifd_entries.push(Entry::new(
            TAG_COMPRESSION,
            DT_SHORT,
            1u64,
            COMPRESS_DEFLATE as u64,
        ));
    } else {
        ifd_entries.push(Entry::new(
            TAG_COMPRESSION,
            DT_SHORT,
            1u64,
            COMPRESS_NONE as u64,
        ));
    }

    // PhotometricInterpretation tag (262)
    let pi = match r.configs.photometric_interp {
        PhotometricInterpretation::Continuous => PI_BLACKISZERO,
        PhotometricInterpretation::Categorical | PhotometricInterpretation::Paletted => PI_PALETTED,
        PhotometricInterpretation::Boolean => PI_BLACKISZERO,
        PhotometricInterpretation::RGB => PI_RGB,
        PhotometricInterpretation::Unknown => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Error while writing GeoTIFF file. Unknown Photometric Interpretation.",
            ));
        }
    };
    ifd_entries.push(Entry::new(
        TAG_PHOTOMETRICINTERPRETATION,
        DT_SHORT,
        1u64,
        pi as u64,
    ));

    // StripOffsets tag (273)
    if !is_big_tiff {
        ifd_entries.push(Entry::new(
            TAG_STRIPOFFSETS,
            DT_LONG,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        if use_compression {
            let mut i = 0;
            println!("Strip Offsets:");
            for val in strip_offsets {
                if i < 100 {
                    println!("{} {}", i, val);
                    i += 1;
                }
                larger_values_data.write_u32(val as u32).expect("Error writing the TIFF strip offsets tag");
            }
        } else {
            let row_length_in_bytes: u32 = r.configs.columns as u32 * total_bytes_per_pixel as u32;
            for i in 0..r.configs.rows as u32 {
                larger_values_data.write_u32(header_size as u32 + row_length_in_bytes * i).expect("Error writing the TIFF strip offsets tag");
            }
        }
    } else {
        ifd_entries.push(Entry::new(
            TAG_STRIPOFFSETS,
            DT_TIFF_LONG8,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        if use_compression {
            for val in strip_offsets {
                larger_values_data.write_u64(val).expect("Error writing the TIFF strip offsets tag");
            }
        } else {
            let row_length_in_bytes: u64 = r.configs.columns as u64 * total_bytes_per_pixel as u64;
            for i in 0..r.configs.rows as u64 {
                larger_values_data.write_u64(header_size + row_length_in_bytes * i).expect("Error writing the TIFF strip offsets");
            }
        }
    }

    // SamplesPerPixel tag (277)
    ifd_entries.push(Entry::new(
        TAG_SAMPLESPERPIXEL,
        DT_SHORT,
        1u64,
        samples_per_pixel as u64,
    ));

    // RowsPerStrip tag (278)
    ifd_entries.push(Entry::new(TAG_ROWSPERSTRIP, DT_SHORT, 1u64, 1u64));

    // StripByteCounts tag (279)
    if !is_big_tiff {
        ifd_entries.push(Entry::new(
            TAG_STRIPBYTECOUNTS,
            DT_LONG,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        let total_bytes_per_pixel = match r.configs.data_type {
            DataType::I8 | DataType::U8 => 1u32,
            DataType::I16 | DataType::U16 => 2u32,
            DataType::I32 | DataType::U32 | DataType::F32 => 4u32,
            DataType::I64 | DataType::U64 | DataType::F64 => 8u32,
            DataType::RGB24 => 3u32,
            DataType::RGBA32 => 4u32,
            DataType::RGB48 => 6u32,
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
            }
        };
        if use_compression {
            let mut i = 0;
            println!("Strip Byte Counts:");
            for val in strip_byte_counts {
                if i < 100 {
                    println!("{} {}", i, val);
                    i += 1;
                }
                larger_values_data.write_u32(val as u32).expect("Error writing the TIFF strip byte counts tag");
            }
        } else {
            let row_length_in_bytes: u32 = r.configs.columns as u32 * total_bytes_per_pixel;
            for _ in 0..r.configs.rows as u32 {
                larger_values_data.write_u32(row_length_in_bytes).expect("Error writing the TIFF strip byte counts tag");
            }
        }
    } else {
        ifd_entries.push(Entry::new(
            TAG_STRIPBYTECOUNTS,
            DT_TIFF_LONG8,
            r.configs.rows as u64,
            larger_values_data.len() as u64,
        ));
        let total_bytes_per_pixel = match r.configs.data_type {
            DataType::I8 | DataType::U8 => 1u64,
            DataType::I16 | DataType::U16 => 2u64,
            DataType::I32 | DataType::U32 | DataType::F32 => 4u64,
            DataType::I64 | DataType::U64 | DataType::F64 => 8u64,
            DataType::RGB24 => 3u64,
            DataType::RGBA32 => 4u64,
            DataType::RGB48 => 6u64,
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
            }
        };
        if use_compression {
            for val in strip_byte_counts {
                larger_values_data.write_u64(val).expect("Error writing the TIFF strip byte counts tag");
            }
        } else {
            let row_length_in_bytes: u64 = r.configs.columns as u64 * total_bytes_per_pixel;
            for _ in 0..r.configs.rows as u32 {
                larger_values_data.write_u64(row_length_in_bytes).expect("Error writing the TIFF strip byte counts tag");
            }
        }
    }

    // There is currently no support for storing the image resolution, so give a bogus value of 72x72 dpi.
    // XResolution tag (282)
    ifd_entries.push(Entry::new(
        TAG_XRESOLUTION,
        DT_RATIONAL,
        1u64,
        larger_values_data.len() as u64,
    ));
    larger_values_data.write_u32(72u32)?;
    larger_values_data.write_u32(1u32)?;

    // YResolution tag (283)
    ifd_entries.push(Entry::new(
        TAG_YRESOLUTION,
        DT_RATIONAL,
        1u64,
        larger_values_data.len() as u64,
    ));
    larger_values_data.write_u32(72u32)?;
    larger_values_data.write_u32(1u32)?;

    // ResolutionUnit tag (296)
    ifd_entries.push(Entry::new(TAG_RESOLUTIONUNIT, DT_SHORT, 1u64, 2u64));

    // Software tag (305)
    let software = "WhiteboxTools".to_owned();
    let mut soft_bytes = software.into_bytes();
    soft_bytes.push(0);
    ifd_entries.push(Entry::new(
        TAG_SOFTWARE,
        DT_ASCII,
        soft_bytes.len() as u64,
        larger_values_data.len() as u64,
    ));
    larger_values_data.write_bytes(&soft_bytes)?;

    if samples_per_pixel == 4 {
        // ExtraSamples tag (338)
        ifd_entries.push(Entry::new(TAG_EXTRASAMPLES, DT_SHORT, 1u64, 2u64));
    }

    // SampleFormat tag (339)
    let samples_format = match r.configs.data_type {
        DataType::U8 | DataType::U16 | DataType::U32 | DataType::U64 => 1u16,
        DataType::I8 | DataType::I16 | DataType::I32 | DataType::I64 => 2u16,
        DataType::F32 | DataType::F64 => 3u16,
        DataType::RGB24 | DataType::RGBA32 | DataType::RGB48 => 1u16,
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
        }
    };
    if samples_per_pixel == 1 {
        ifd_entries.push(Entry::new(
            TAG_SAMPLEFORMAT,
            DT_SHORT,
            samples_per_pixel as u64,
            samples_format as u64,
        ));
    } else {
        ifd_entries.push(Entry::new(
            TAG_SAMPLEFORMAT,
            DT_SHORT,
            samples_per_pixel as u64,
            larger_values_data.len() as u64,
        ));
        for _ in 0..samples_per_pixel {
            larger_values_data.write_u16(samples_format)?;
        }
    }

    // ModelPixelScaleTag tag (33550)
    if r.configs.model_pixel_scale[0] == 0f64
        && r.configs.model_tiepoint.is_empty()
        && r.configs.model_transformation[0] == 0f64
    {
        ifd_entries.push(Entry::new(
            TAG_MODELPIXELSCALETAG,
            DT_DOUBLE,
            3u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_f64(r.configs.resolution_x)?;
        larger_values_data.write_f64(r.configs.resolution_y)?;
        larger_values_data.write_f64(0f64)?;
    } else if r.configs.model_pixel_scale[0] != 0f64 {
        ifd_entries.push(Entry::new(
            TAG_MODELPIXELSCALETAG,
            DT_DOUBLE,
            3u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_f64(r.configs.model_pixel_scale[0])?;
        larger_values_data.write_f64(r.configs.model_pixel_scale[1])?;
        larger_values_data.write_f64(r.configs.model_pixel_scale[2])?;
    }

    if r.configs.model_tiepoint.is_empty() && r.configs.model_transformation[0] == 0f64 {
        // ModelTiepointTag tag (33922)
        ifd_entries.push(Entry::new(
            TAG_MODELTIEPOINTTAG,
            DT_DOUBLE,
            6u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_f64(0f64)?; // I
        larger_values_data.write_f64(0f64)?; // J
        larger_values_data.write_f64(0f64)?; // K
        larger_values_data.write_f64(r.configs.west)?; // X
        larger_values_data.write_f64(r.configs.north)?; // Y
        larger_values_data.write_f64(0f64)?; // Z
    } else if !r.configs.model_tiepoint.is_empty() {
        // ModelTiepointTag tag (33922)
        ifd_entries.push(Entry::new(
            TAG_MODELTIEPOINTTAG,
            DT_DOUBLE,
            r.configs.model_tiepoint.len() as u64,
            larger_values_data.len() as u64,
        ));
        for i in 0..r.configs.model_tiepoint.len() {
            larger_values_data.write_f64(r.configs.model_tiepoint[i])?;
        }
    }

    if r.configs.model_transformation[0] != 0f64 {
        // ModelTransformationTag tag (33920)
        ifd_entries.push(Entry::new(
            TAG_MODELTRANSFORMATIONTAG,
            DT_DOUBLE,
            16u64,
            larger_values_data.len() as u64,
        ));
        for i in 0..16 {
            larger_values_data.write_f64(r.configs.model_transformation[i])?;
        }
    }

    // TAG_GDAL_NODATA tag (42113)
    let nodata_str = format!("{}", r.configs.nodata);
    let mut nodata_bytes = nodata_str.into_bytes();
    if !is_big_tiff {
        // we buffer this string with spaces to ensure that it is
        // long enough to be printed to larger_values_data.
        if nodata_bytes.len() < 4 {
            for _ in 0..(4 - nodata_bytes.len()) {
                nodata_bytes.push(32);
            }
        }
        if nodata_bytes.len() % 2 == 0 {
            nodata_bytes.push(32);
        }
        nodata_bytes.push(0);
        ifd_entries.push(Entry::new(
            TAG_GDAL_NODATA,
            DT_ASCII,
            nodata_bytes.len() as u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_bytes(&nodata_bytes)?;
    } else {
        // we buffer this string with spaces to ensure that it is
        // long enough to be printed to larger_values_data.
        if nodata_bytes.len() < 8 {
            for _ in 0..(8 - nodata_bytes.len()) {
                nodata_bytes.push(32);
            }
        }
        if nodata_bytes.len() % 2 == 0 {
            nodata_bytes.push(32);
        }
        nodata_bytes.push(0);
        ifd_entries.push(Entry::new(
            TAG_GDAL_NODATA,
            DT_ASCII,
            nodata_bytes.len() as u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_bytes(&nodata_bytes)?;
    }

    let kw_map = get_keyword_map();
    let geographic_type_map = match kw_map.get(&2048u16) {
        Some(map) => map,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Error generating geographic type map.",
            ))
        }
    };
    let projected_cs_type_map = match kw_map.get(&3072u16) {
        Some(map) => map,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Error generating projected coordinate system type map.",
            ))
        }
    };

    //let key_map = get_keys_map();
    let mut gk_entries: Vec<GeoKeyEntry> = vec![];
    let mut ascii_params = String::new(); //: Vec<u8> = vec![];
    let double_params: Vec<f64> = vec![];
    if geographic_type_map.contains_key(&r.configs.epsg_code) {
        // tGTModelTypeGeoKey (1024)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTMODELTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: 2u16,
        });

        // GTRasterTypeGeoKey (1025)
        if r.configs.pixel_is_area {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 1u16,
            });
        } else {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 2u16,
            });
        }

        // tGTCitationGeoKey (1026)
        let mut v = String::from(
            geographic_type_map
                .get(&r.configs.epsg_code)
                .unwrap()
                .clone(),
        );
        v.push_str("|");
        v = v.replace("_", " ");
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTCITATIONGEOKEY,
            location: 34737u16,
            count: v.len() as u16,
            value_offset: ascii_params.len() as u16,
        });
        ascii_params.push_str(&v);

        // tGeographicTypeGeoKey (2048)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GEOGRAPHICTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: r.configs.epsg_code,
        });

        if r.configs.z_units.to_lowercase() != "not specified" {
            // VerticalUnitsGeoKey (4099)
            let units = r.configs.z_units.to_lowercase();
            if units.contains("met") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9001u16,
                });
            } else if units.contains("ft") | units.contains("feet") | units.contains("foot") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9002u16,
                });
            }
        }
    } else if projected_cs_type_map.contains_key(&r.configs.epsg_code) {
        // tGTModelTypeGeoKey (1024)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTMODELTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: 1u16,
        });

        // GTRasterTypeGeoKey (1025)
        if r.configs.pixel_is_area {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 1u16,
            });
        } else {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 2u16,
            });
        }

        // tProjectedCSTypeGeoKey (3072)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_PROJECTEDCSTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: r.configs.epsg_code,
        });

        // PCSCitationGeoKey (3073)
        let mut v = String::from(
            projected_cs_type_map
                .get(&r.configs.epsg_code)
                .unwrap()
                .clone(),
        );
        v.push_str("|");
        v = v.replace("_", " ");
        gk_entries.push(GeoKeyEntry {
            tag: 3073u16,
            location: 34737u16,
            count: v.len() as u16,
            value_offset: ascii_params.len() as u16,
        });
        ascii_params.push_str(&v);

        if r.configs.xy_units.to_lowercase() != "not specified" {
            // ProjLinearUnitsGeoKey (3076)
            let units = r.configs.xy_units.to_lowercase();
            if units.contains("met") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_PROJLINEARUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9001u16,
                });
            } else if units.contains("ft") | units.contains("feet") | units.contains("foot") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_PROJLINEARUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9002u16,
                });
            }
        }

        if r.configs.z_units.to_lowercase() != "not specified" {
            // VerticalUnitsGeoKey (4099)
            let units = r.configs.z_units.to_lowercase();
            if units.contains("met") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9001u16,
                });
            } else if units.contains("ft") | units.contains("feet") | units.contains("foot") {
                gk_entries.push(GeoKeyEntry {
                    tag: TAG_VERTICALUNITSGEOKEY,
                    location: 0u16,
                    count: 1u16,
                    value_offset: 9002u16,
                });
            }
        }
    } else {
        // we don't know much about the coordinate system used.

        // tGTModelTypeGeoKey (1024)
        gk_entries.push(GeoKeyEntry {
            tag: TAG_GTMODELTYPEGEOKEY,
            location: 0u16,
            count: 1u16,
            value_offset: 0u16,
        });

        // GTRasterTypeGeoKey (1025)
        if r.configs.pixel_is_area {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 1u16,
            });
        } else {
            gk_entries.push(GeoKeyEntry {
                tag: TAG_GTRASTERTYPEGEOKEY,
                location: 0u16,
                count: 1u16,
                value_offset: 2u16,
            });
        }
    }

    if r.configs.geo_key_directory.is_empty() {
        // create the GeoKeyDirectoryTag tag (34735)
        ifd_entries.push(Entry::new(
            TAG_GEOKEYDIRECTORYTAG,
            DT_SHORT,
            (4 + gk_entries.len() * 4) as u64,
            larger_values_data.len() as u64,
        ));
        larger_values_data.write_u16(1u16)?; // KeyDirectoryVersion
        larger_values_data.write_u16(1u16)?; // KeyRevision
        larger_values_data.write_u16(0u16)?; // MinorRevision
        larger_values_data.write_u16(gk_entries.len() as u16)?; // NumberOfKeys

        for entry in gk_entries {
            larger_values_data.write_u16(entry.tag)?; // KeyID
            larger_values_data.write_u16(entry.location)?; // TIFFTagLocation
            larger_values_data.write_u16(entry.count)?; // Count
            larger_values_data.write_u16(entry.value_offset)?; // Value_Offset
        }

        if double_params.len() > 0 {
            // create the GeoDoubleParamsTag tag (34736)
            ifd_entries.push(Entry::new(
                TAG_GEODOUBLEPARAMSTAG,
                DT_DOUBLE,
                double_params.len() as u64,
                larger_values_data.len() as u64,
            ));
            for double_val in double_params {
                larger_values_data.write_f64(double_val)?;
            }
        }

        if ascii_params.len() > 0 {
            // create the GeoAsciiParamsTag tag (34737)
            let mut ascii_params_bytes = ascii_params.into_bytes();
            ascii_params_bytes.push(0);
            ifd_entries.push(Entry::new(
                TAG_GEOASCIIPARAMSTAG,
                DT_ASCII,
                ascii_params_bytes.len() as u64,
                larger_values_data.len() as u64,
            ));
            if ascii_params_bytes.len() % 2 == 1 {
                // it has to end on a word so that the next value starts on a word
                ascii_params_bytes.push(0);
            }
            larger_values_data.write_bytes(&ascii_params_bytes)?;
        }
    } else {
        // let num_keys = (r.configs.geo_key_directory.len() - 4) / 4;
        // output the GeoKeyDirectoryTag tag (34735)
        ifd_entries.push(Entry::new(
            TAG_GEOKEYDIRECTORYTAG,
            DT_SHORT,
            r.configs.geo_key_directory.len() as u64,
            larger_values_data.len() as u64,
        ));
        for val in &r.configs.geo_key_directory {
            larger_values_data.write_u16(*val)?;
        }

        if r.configs.geo_double_params.len() > 0 {
            // create the GeoDoubleParamsTag tag (34736)
            ifd_entries.push(Entry::new(
                TAG_GEODOUBLEPARAMSTAG,
                DT_DOUBLE,
                r.configs.geo_double_params.len() as u64,
                larger_values_data.len() as u64,
            ));
            for double_val in &r.configs.geo_double_params {
                larger_values_data.write_f64(*double_val)?;
            }
        }

        if !r.configs.geo_ascii_params.is_empty() {
            // create the GeoAsciiParamsTag tag (34737)
            let mut ascii_params_bytes = r.configs.geo_ascii_params.clone().into_bytes();
            ascii_params_bytes.push(0);
            ifd_entries.push(Entry::new(
                TAG_GEOASCIIPARAMSTAG,
                DT_ASCII,
                ascii_params_bytes.len() as u64,
                larger_values_data.len() as u64,
            ));
            if ascii_params_bytes.len() % 2 == 1 {
                // it has to end on a word so that the next value starts on a word
                ascii_params_bytes.push(0);
            }
            larger_values_data.write_bytes(&ascii_params_bytes)?;
        }
    }

    ///////////////////
    // Write the IFD //
    ///////////////////

    // Number of Directory Entries.
    if !is_big_tiff {
        bow.write_u16(ifd_entries.len() as u16)?;

        // Sort the IFD entries
        ifd_entries.sort_by(|a, b| a.tag.cmp(&b.tag));

        // Write the entries
        let ifd_length = 2u64 + ifd_entries.len() as u64 * 12u64 + 4u64;

        for ifde in ifd_entries {
            bow.write_u16(ifde.tag)?; // Tag
            bow.write_u16(ifde.ifd_type)?; // Field type
            bow.write_u32(ifde.num_values as u32)?; // Num of values
            if ifde.ifd_type == DT_SHORT && ifde.num_values == 1 {
                // it's a value
                bow.write_u16(ifde.offset as u16)?; // Value
                bow.write_u16(0u16)?; // Fill the remaining 2 right bytes of the u32
            } else if ifde.ifd_type == DT_LONG && ifde.num_values == 1 {
                // it's a value
                bow.write_u32(ifde.offset as u32)?;
            } else if ifde.ifd_type == DT_SHORT && ifde.num_values == 2 {
                // I'm not really sure about this one. Two shorts will fit in the value_offset, but will they be interpreted correctly?
                bow.write_u32(ifde.offset as u32)?; // Value
            } else {
                // it's an offset
                bow.write_u32(ifd_start as u32 + ifd_length as u32 + ifde.offset as u32)?;
            }
        }

        // 4-byte offset of the next IFD; Note, only single image TIFFs are currently supported
        // and therefore, this will always be set to '0'.
        bow.write_u32(0u32)?;
    } else {
        bow.write_u64(ifd_entries.len() as u64)?;

        // Sort the IFD entries
        ifd_entries.sort_by(|a, b| a.tag.cmp(&b.tag));

        // Write the entries
        let ifd_length = 8u64 + ifd_entries.len() as u64 * 20u64 + 8u64;

        for ifde in ifd_entries {
            bow.write_u16(ifde.tag)?; // Tag
            bow.write_u16(ifde.ifd_type)?; // Field type
            bow.write_u64(ifde.num_values)?; // Num of values
            if ifde.ifd_type == DT_SHORT && ifde.num_values == 1 {
                // it's a value
                bow.write_u16(ifde.offset as u16)?; // Value
                bow.write_u16(0u16)?; // Fill the remaining bytes of the u64
                bow.write_u32(0u32)?; // Fill the remaining bytes of the u64
            } else if ifde.ifd_type == DT_SHORT && ifde.num_values == 2 {
                // I'm not really sure about this one. Two shorts will fit in the value_offset, but will they be interpreted correctly?
                bow.write_u32(ifde.offset as u32)?; // Value
                bow.write_u32(0u32)?; // Fill the remaining bytes of the u64
            } else if ifde.ifd_type == DT_LONG && ifde.num_values == 1 {
                // it's a value
                bow.write_u32(ifde.offset as u32)?;
                bow.write_u32(0u32)?; // Fill the remaining bytes of the u64
            } else if (ifde.ifd_type == DT_LONG && ifde.num_values == 2)
                || (ifde.ifd_type == DT_TIFF_LONG8 && ifde.num_values == 1)
            {
                // it's a value
                bow.write_u64(ifde.offset)?;
            } else {
                // it's an offset
                bow.write_u64(ifd_start + ifd_length + ifde.offset)?;
            }
        }

        // 4-byte offset of the next IFD; Note, only single image TIFFs are currently supported
        // and therefore, this will always be set to '0'.
        bow.write_u64(0u64)?;
    }

    //////////////////////////////////
    // Write the larger_values_data //
    //////////////////////////////////
    bow.write_bytes(&(larger_values_data.into_inner().into_inner()))?;

    Ok(())
}

*/

// An implementation of a PackBits reader
pub fn packbits_decoder(input_data: Vec<u8>) -> Vec<u8> {
    let mut output_data = vec![];
    let mut i: usize = 0;
    while i < input_data.len() {
        let hex = input_data[i];
        if hex >= 128 {
            let hex2 = (256i16 - hex as i16) as u8;
            for _ in 0..(hex2 + 1) {
                output_data.push(input_data[i + 1]);
            }
            i += 1;
        } else {
            for j in 0..(hex + 1) {
                output_data.push(input_data[i + j as usize + 1]);
            }
            i += hex as usize + 1;
        }
        i += 1;
    }
    output_data
}

pub fn write_u8<W: Write>(writer: &mut BufWriter<W>, value: u8) -> Result<(), Error> {
    writer.write_u8(value)
}

pub fn write_i8<W: Write>(writer: &mut BufWriter<W>, value: i8) -> Result<(), Error> {
    writer.write_i8(value)
}

pub fn write_bytes<W: Write>(writer: &mut BufWriter<W>, bytes: &[u8]) -> Result<(), Error> {
    writer.write_all(bytes)
}

pub fn write_u16<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: u16,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_u16::<LittleEndian>(value)
    } else {
        writer.write_u16::<BigEndian>(value)
    }
}

pub fn write_i16<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: i16,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_i16::<LittleEndian>(value)
    } else {
        writer.write_i16::<BigEndian>(value)
    }
}

pub fn write_u32<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: u32,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_u32::<LittleEndian>(value)
    } else {
        writer.write_u32::<BigEndian>(value)
    }
}

pub fn write_i32<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: i32,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_i32::<LittleEndian>(value)
    } else {
        writer.write_i32::<BigEndian>(value)
    }
}

pub fn write_u64<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: u64,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_u64::<LittleEndian>(value)
    } else {
        writer.write_u64::<BigEndian>(value)
    }
}

pub fn write_i64<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: i64,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_i64::<LittleEndian>(value)
    } else {
        writer.write_i64::<BigEndian>(value)
    }
}

pub fn write_f32<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: f32,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_f32::<LittleEndian>(value)
    } else {
        writer.write_f32::<BigEndian>(value)
    }
}

pub fn write_f64<W: Write>(
    writer: &mut BufWriter<W>,
    endianness: Endianness,
    value: f64,
) -> Result<(), Error> {
    if endianness == Endianness::LittleEndian {
        writer.write_f64::<LittleEndian>(value)
    } else {
        writer.write_f64::<BigEndian>(value)
    }
}
