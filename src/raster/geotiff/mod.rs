#![allow(unused_assignments, dead_code)]
pub mod geokeys;

use std::collections::HashMap;
use std::io::Error;
use std::io::ErrorKind;
use std::fmt;
use std::default::Default;
use std::cmp::min;
use std::io::BufWriter;
use std::io::prelude::*;
use std::f64;
use std::fs::File;
use std::fs;
use std::mem;
use raster::*;
use raster::geotiff::geokeys::*;
use io_utils::byte_order_reader::ByteOrderReader;
use io_utils::byte_order_reader::Endianness;

pub fn read_geotiff<'a>(file_name: &'a String, configs: &'a mut RasterConfigs, data: &'a mut Vec<f64>) -> Result<(), Error> {
    let mut f = File::open(file_name.clone())?;

    let metadata = fs::metadata(file_name.clone())?;
    let file_size: usize = metadata.len() as usize;
    let mut buffer = vec![0; file_size];

    // read the file's bytes into a buffer
    f.read(&mut buffer)?;

    //let byte_order = LittleEndian::read_u16(&buffer[0..2]);
    match &buffer[0..2] { //byte_order { //LittleEndian::read_u16(&buffer[0..2]) {
        b"II" => configs.endian = Endianness::LittleEndian,
        b"MM" => configs.endian = Endianness::BigEndian,
        _ => return Err(Error::new(ErrorKind::InvalidData, "Incorrect TIFF header."))
    }

    let mut th = ByteOrderReader::new(buffer, configs.endian);
    th.seek(2);

    match th.read_u16() {
        42 => (), // do nothing
        43 => return Err(Error::new(ErrorKind::InvalidData, "The BigTiff format is not currently supported.")),
        _ => return Err(Error::new(ErrorKind::InvalidData, "Incorrect TIFF header."))
    }

    let mut ifd_offset = th.read_u32() as usize;

    let mut ifd_map = HashMap::new();

    let mut geokeys: GeoKeys = Default::default();
    let mut cur_pos: usize;
    while ifd_offset > 0 {
        th.seek(ifd_offset);
        let num_directories = th.read_u16();
        for _ in 0..num_directories {
            let tag_id = th.read_u16();
            let field_type = th.read_u16();
            let num_values = th.read_u32();
            let value_offset = th.read_u32();
            let data_size = match field_type {
                1u16 | 2u16 | 6u16 | 7u16 => 1,
                3u16 | 8u16 => 2,
                4u16 | 9u16 | 11u16 => 4,
                5u16 | 10u16 | 12u16 => 8,
                _ => return Err(Error::new(ErrorKind::InvalidInput, "Error reading the IFDs."))
            };

            // read the tag data
            let mut data: Vec<u8> = vec![];
            if (data_size * num_values) > 4 {
                // the values are stored at the offset location
                cur_pos = th.pos;
                th.seek(value_offset as usize);
                for _ in 0..num_values*data_size {
                    data.push(th.read_u8());
                }
                th.seek(cur_pos);
            } else {
                // the value(s) are contained in the offset
                cur_pos = th.pos;
                th.seek(cur_pos - 4);
                for _ in 0..num_values*data_size {
                    data.push(th.read_u8());
                }
                th.seek(cur_pos);
            }

            let ifd = IfdDirectory::new(
                tag_id,
                field_type,
                num_values,
                value_offset,
                data,
                configs.endian
            );
            ifd_map.insert(tag_id, ifd.clone());
        }
        ifd_offset = th.read_u32() as usize;
    }

    configs.columns = match ifd_map.get(&256) {
        Some(ifd) => {
            // The 256 tag can be either u16 or u32 type
            if ifd.ifd_type == 3 {
                ifd.interpret_as_u16()[0] as usize
            } else {
                ifd.interpret_as_u32()[0] as usize
            }
        },
        _ => return Err(Error::new(ErrorKind::InvalidData, "The raster Columns value was not read correctly")),
    };

    configs.rows = match ifd_map.get(&257) {
        Some(ifd) => {
            // The 257 tag can be either u16 or u32 type
            if ifd.ifd_type == 3 {
                ifd.interpret_as_u16()[0] as usize
            } else {
                ifd.interpret_as_u32()[0] as usize
            }
        },
        _ => return Err(Error::new(ErrorKind::InvalidData, "The raster Rows value was not read correctly")),
    };

    //data = vec![0.0f64; configs.rows * configs.columns];
    data.clear();
    for _ in 0..configs.rows*configs.columns {
        data.push(0.0f64);
    }

    let bits_per_sample = match ifd_map.get(&258) {
        Some(ifd) => ifd.interpret_as_u16(),
        _ => return Err(Error::new(ErrorKind::InvalidData, "The raster BitsPerSample value was not read correctly")),
    };

    let compression = match ifd_map.get(&259) {
        Some(ifd) => ifd.interpret_as_u16()[0],
        _ => return Err(Error::new(ErrorKind::InvalidData, "The raster Compression method value was not read correctly")),
    };

    let photometric_interp = match ifd_map.get(&262) {
        Some(ifd) => ifd.interpret_as_u16()[0],
        _ => return Err(Error::new(ErrorKind::InvalidData, "The raster PhotometricInterpretation value was not read correctly")),
    };

    // let num_samples = match ifd_map.get(&277) {
    //     Some(ifd) => ifd.interpret_as_u16()[0],
    //     _ => 0,
    // };

    let extra_samples = match ifd_map.get(&338) {
        Some(ifd) => ifd.interpret_as_u16()[0],
        _ => 0,
    };

    let sample_format = match ifd_map.get(&339) {
        Some(ifd) => ifd.interpret_as_u16(),
        _ => [0].to_vec(),
    };

    match ifd_map.get(&34735) {
        Some(ifd) => geokeys.add_key_directory(&ifd.data),
        _ => return Err(Error::new(ErrorKind::InvalidData, "The TIFF file does not contain geokeys")),
    };

    match ifd_map.get(&34736) {
        Some(ifd) => geokeys.add_double_params(&ifd.data),
        _ => {},
    };

    match ifd_map.get(&34737) {
        Some(ifd) => geokeys.add_ascii_params(&ifd.data),
        _ => {},
    };

    let geokeys_map = geokeys.get_ifd_map(configs.endian);

    let model_tiepoints = match ifd_map.get(&33922) {
        Some(ifd) => ifd.interpret_as_f64(),
        _ => vec![0.0],
    };

    let model_pixel_scale = match ifd_map.get(&33550) {
        Some(ifd) => ifd.interpret_as_f64(),
        _ => vec![0.0],
    };

    if model_tiepoints.len() == 6 && model_pixel_scale.len() == 3 {
        configs.resolution_x = model_pixel_scale[0];
        configs.resolution_y = model_pixel_scale[1];
        configs.west = model_tiepoints[3];
        configs.east = configs.west + configs.resolution_x * configs.columns as f64;
        configs.north = model_tiepoints[4];
        configs.south = configs.north - configs.resolution_y * configs.rows as f64;
    }

    // Get the EPSG code
    if geokeys_map.contains_key(&2048) {
        configs.epsg_code = geokeys_map.get(&2048).unwrap().interpret_as_u16()[0];
    }


    // Determine the image mode.
    let kw_map = get_keyword_map();
    let photomet_map = kw_map.get(&262).unwrap();
    let photomet_str: String = photomet_map.get(&photometric_interp).unwrap().to_string();
    let mode: ImageMode;
    let mut palette = vec![];
    if photomet_str == "RGB" {
        if bits_per_sample[0] == 16 {
            if bits_per_sample[1] != 16 || bits_per_sample[2] != 16 {
                return Err(Error::new(ErrorKind::InvalidData, "Wrong number of samples for 16bit RGB."));
            }
        } else {
            if bits_per_sample[0] != 8 || bits_per_sample[1] != 8 || bits_per_sample[2] != 8 {
                return Err(Error::new(ErrorKind::InvalidData, "Wrong number of samples for 8bit RGB."));
            }
        }
        // RGB images normally have 3 samples per pixel.
        // If there are more, ExtraSamples (p. 31-32 of the spec)
        // gives their meaning (usually an alpha channel).
        // This implementation does not support extra samples
        // of an unspecified type.
        mode = match bits_per_sample.len() {
            3 => ImageMode::RGB,
            4 => {
                match extra_samples {
                    1 => ImageMode::RGBA,
                    2 => ImageMode::NRGBA,
                    _ => return Err(Error::new(ErrorKind::InvalidData, "Wrong number of samples for RGB."))
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "Wrong number of samples for RGB."))
        };
    } else if photomet_str == "Paletted" {
        mode = ImageMode::Paletted;
        // retreive the palette colour data
        let color_map = match ifd_map.get(&320) {
            Some(ifd) => ifd.interpret_as_u16(),
            _ => return Err(Error::new(ErrorKind::InvalidData, "Colour map not present in Paletted TIFF.")),
        };
        let num_colors = color_map.len() / 3;
        if color_map.len()%3 != 0 || num_colors <= 0 || num_colors > 256 {
            return Err(Error::new(ErrorKind::InvalidData, "bad ColorMap length"));
        }
        for i in 0..num_colors {
            // colours in the colour map are given in 16-bit channels
            // and need to be rescaled to an 8-bit format.
            let red = (color_map[i] as f64 / 65535.0 * 255.0) as u32;
            let green = (color_map[i+num_colors] as f64 / 65535.0 * 255.0) as u32;
            let blue = (color_map[i+2*num_colors] as f64 / 65535.0 * 255.0) as u32;
            let a = 255u32;
            let val = ((a << 24) | (red << 16) | (green << 8) | blue) as u32;
            palette.push(val);
        }
    } else if photomet_str == "WhiteIsZero" {
        mode = ImageMode::GrayInvert;
    } else if photomet_str == "BlackIsZero" {
        mode = ImageMode::Gray;
    } else {
        return Err(Error::new(ErrorKind::InvalidData, "Unsupported image format."))
    }

    let width = configs.columns;
    let height = configs.rows;

    let mut block_padding = false;
    let mut block_width = configs.columns;
    let block_height; // = configs.rows;
    let mut blocks_across = 1;
    let blocks_down; // = 1;

    let block_offsets: Vec<u32>; //  = vec![];
    let block_counts: Vec<u32>; // = vec![];

    if ifd_map.contains_key(&322) {
        block_padding = true;

        block_width = match ifd_map.get(&322) {
            Some(ifd) => {
                // The 322 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    ifd.interpret_as_u16()[0] as usize
                } else {
                    ifd.interpret_as_u32()[0] as usize
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "The TileWidth value was not read correctly")),
        };

        block_height = match ifd_map.get(&323) {
            Some(ifd) => {
                // The 323 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    ifd.interpret_as_u16()[0] as usize
                } else {
                    ifd.interpret_as_u32()[0] as usize
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "The TileLength value was not read correctly")),
        };

        blocks_across = (width + block_width - 1) / block_width;
        blocks_down = (height + block_height - 1) / block_height;

        block_offsets = match ifd_map.get(&324) {
            Some(ifd) => ifd.interpret_as_u32(),
            _ => return Err(Error::new(ErrorKind::InvalidData, "The raster BitsPerSample value was not read correctly")),
        };

        block_counts = match ifd_map.get(&325) {
            Some(ifd) => {
                // The 325 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    let ifd_data = ifd.interpret_as_u16();
                    let mut ret: Vec<u32> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u32);
                    }
                    ret
                } else {
                    ifd.interpret_as_u32()
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "The TileLength value was not read correctly")),
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
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "The RowsPerStrip value was not read correctly")),
        };

        blocks_down = (height + block_height - 1) / block_height;

        block_offsets = match ifd_map.get(&273) {
            Some(ifd) => {
                // The 273 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    let ifd_data = ifd.interpret_as_u16();
                    let mut ret: Vec<u32> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u32);
                    }
                    ret
                } else {
                    ifd.interpret_as_u32()
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "The raster StripOffsets value was not read correctly")),
        };

        block_counts = match ifd_map.get(&279) {
            Some(ifd) => {
                // The 279 tag can be either u16 or u32 type
                if ifd.ifd_type == 3 {
                    let ifd_data = ifd.interpret_as_u16();
                    let mut ret: Vec<u32> = vec![];
                    for val in ifd_data.iter() {
                        ret.push(*val as u32);
                    }
                    ret
                } else {
                    ifd.interpret_as_u32()
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "The raster StripByteCounts value was not read correctly")),
        };
    }

    for i in 0..blocks_across {
        let mut blk_w = block_width;
        if !block_padding && i == blocks_across-1 && width%block_width != 0 {
            blk_w = width % block_width;
        }
        for j in 0..blocks_down {
            let mut blk_h = block_height;
            if !block_padding && j == blocks_down-1 && height%block_height != 0 {
                blk_h = height % block_height;
            }
            let offset = block_offsets[j*blocks_across+i] as usize;
            let n = block_counts[j*blocks_across+i] as usize;
            let mut buf: Vec<u8> = vec![];
            match compression {
                COMPRESS_NONE => { // no compression
                    buf = th.buffer[offset..(offset+n)].to_vec();
                }
                COMPRESS_PACKBITS => {
                    buf = packbits_decoder(th.buffer[offset..(offset+n)].to_vec());
                },
                _ => {

                }
            }
            let mut bor = ByteOrderReader::new(buf, configs.endian);


            let xmin = i * block_width;
            let ymin = j * block_height;
            let mut xmax = xmin + blk_w;
            let mut ymax = ymin + blk_h;

            xmax = min(xmax, width);
            ymax = min(ymax, height);

            let mut off = 0;

            match mode {
                ImageMode::GrayInvert | ImageMode::Gray => {
                    match sample_format[0] {
                        1 => { // unsigned integer
                            match bits_per_sample[0] {
                                8 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let i = y*width + x;
                                                data[i] = bor.read_u8() as f64; //buf[off] as f64;
                                                off += 1;
                                            }
                                        }
                                    }
                                },
                                16 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let value = bor.read_u16(); // g.ByteOrder.Uint16(g.buf[g.off : g.off+2])
                                                let i = y*width + x;
                                                data[i] = value as f64;
                                                off += 2;
                                            }
                                        }
                                    }
                                },
                                32 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let value = bor.read_u32();
                                                let i = y*width + x;
                                                data[i] = value as f64;
                                                off += 4;
                                            }
                                        }
                                    }
                                },
                                64 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let value = bor.read_u64();
                                                let i = y*width + x;
                                                data[i] = value as f64;
                                                off += 8;
                                            }
                                        }
                                    }
                                },
                                _ => return Err(Error::new(ErrorKind::InvalidData, "The raster was not read correctly")),
                            }
                        },
                        2 => { // signed integer
                            match bits_per_sample[0] {
                                8 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let i = y*width + x;
                                                data[i] = bor.read_i8() as f64;
                                                off += 1;
                                            }
                                        }
                                    }
                                },
                                16 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let value = bor.read_i16();
                                                let i = y*width + x;
                                                data[i] = value as f64;
                                                off += 2;
                                            }
                                        }
                                    }
                                },
                                32 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let value = bor.read_i32();
                                                let i = y*width + x;
                                                data[i] = value as f64;
                                                off += 4;
                                            }
                                        }
                                    }
                                },
                                64 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let value = bor.read_i64();
                                                let i = y*width + x;
                                                data[i] = value as f64;
                                                off += 8;
                                            }
                                        }
                                    }
                                },
                                _ => return Err(Error::new(ErrorKind::InvalidData, "The raster was not read correctly")),
                            }
                        },
                        3 => { // floating point
                            match bits_per_sample[0] {
                                32 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            let value = bor.read_f32();
                                            let i = y*width + x;
                                            data[i] = value as f64;
                                            off += 4;
                                        }
                                    }
                                },
                                64 => {
                                    for y in ymin..ymax {
                                        for x in xmin..xmax {
                                            if off <= bor.len() {
                                                let value = bor.read_f64();
                                                let i = y*width + x;
                                                data[i] = value;
                                                off += 8;
                                            }
                                        }
                                    }
                                },
                                _ => return Err(Error::new(ErrorKind::InvalidData, "The raster was not read correctly")),
                            }
                        },
                        _ => return Err(Error::new(ErrorKind::InvalidData, "The raster was not read correctly")),
                    }
                },
                ImageMode::Paletted => {
                    for y in ymin..ymax {
                        for x in xmin..xmax {
                            let i = y*width + x;
                            let value = bor.read_u8() as usize;
                            data[i] = palette[value] as f64;
                        }
                    }
                },
                ImageMode::RGB => {
                    if bits_per_sample[0] == 8 {
                        for y in ymin..ymax {
                            for x in xmin..xmax {
                                let red = bor.read_u8() as u32; //uint32(g.buf[g.off]);
                                let green = bor.read_u8() as u32; //uint32(g.buf[g.off+1]);
                                let blue = bor.read_u8() as u32; //uint32(g.buf[g.off+2]);
                                let a = 255u32;
                                let value = (a << 24) | (red << 16) | (green << 8) | blue;
                                let i = y*width + x;
                                data[i] = value as f64;
                            }
                        }
                    } else if bits_per_sample[0] == 16 {
                        // the spec doesn't talk about 16-bit RGB images so
                        // I'm not sure why I bother with this. They specifically
                        // say that RGB images are 8-bits per channel. Anyhow,
                        // I rescale the 16-bits to an 8-bit channel for simplicity.
                        for y in ymin..ymax {
                            for x in xmin..xmax {
                                let red = (bor.read_u16() as f64 / 65535f64 * 255f64) as u32;
                                let green = (bor.read_u16() as f64 / 65535f64 * 255f64) as u32;
                                let blue = (bor.read_u16() as f64 / 65535f64 * 255f64) as u32;
                                let a = 255u32;
                                let value = (a << 24) | (red << 16) | (green << 8) | blue;
                                let i = y*width + x;
                                data[i] = value as f64;
                            }
                        }
                    } else {
                        return Err(Error::new(ErrorKind::InvalidData, "The raster was not read correctly"));
                    }
                },
                ImageMode::NRGBA | ImageMode::RGBA => {
                    if bits_per_sample[0] == 8 {
                        for y in ymin..ymax {
                            for x in xmin..xmax {
                                let red = bor.read_u8() as u32; //uint32(g.buf[g.off]);
                                let green = bor.read_u8() as u32; //uint32(g.buf[g.off+1]);
                                let blue = bor.read_u8() as u32; //uint32(g.buf[g.off+2]);
                                let a = bor.read_u8() as u32;
                                let value = (a << 24) | (red << 16) | (green << 8) | blue;
                                let i = y*width + x;
                                data[i] = value as f64;
                            }
                        }
                    } else if bits_per_sample[0] == 16 {
                        // the spec doesn't talk about 16-bit RGB images so
                        // I'm not sure why I bother with this. They specifically
                        // say that RGB images are 8-bits per channel. Anyhow,
                        // I rescale the 16-bits to an 8-bit channel for simplicity.
                        for y in ymin..ymax {
                            for x in xmin..xmax {
                                let red = (bor.read_u16() as f64 / 65535f64 * 255f64) as u32;
                                let green = (bor.read_u16() as f64 / 65535f64 * 255f64) as u32;
                                let blue = (bor.read_u16() as f64 / 65535f64 * 255f64) as u32;
                                let a = (bor.read_u16() as f64 / 65535f64 * 255f64) as u32;
                                let value = (a << 24) | (red << 16) | (green << 8) | blue;
                                let i = y*width + x;
                                data[i] = value as f64;
                            }
                        }
                    } else {
                        return Err(Error::new(ErrorKind::InvalidData, "The raster was not read correctly"));
                    }
                },
                _ => return Err(Error::new(ErrorKind::InvalidData, "The raster was not read correctly")),
            }
        }
    }

    // match geokeys_map.get(&1024) {
    //     Some(ifd) => geokeys.add_key_directory(&ifd.data),
    //     _ => return Err(Error::new(ErrorKind::InvalidData, "The TIFF file does not contain geokeys")),
    // };

    let mut map_sorter = vec![];
    for (key, _) in ifd_map.iter() {
        map_sorter.push(key);
        //println!("{}", ifd);
    }
    map_sorter.sort();
    for key in map_sorter.iter() {
        println!("{}", ifd_map.get(&key).unwrap());
    }

    map_sorter.clear();
    for (key, _) in geokeys_map.iter() {
        map_sorter.push(key);
        //println!("{}", ifd);
    }
    for key in map_sorter.iter() {
        println!("{}", geokeys_map.get(&key).unwrap());
    }


    // println!("\nGeoKeys:\n");
    // println!("{}", geokeys.interpret_geokeys());

    Ok(())
}

pub fn write_geotiff<'a>(r: &'a mut Raster) -> Result<(), Error> {
    let f = File::create(r.file_name.clone())?;
    let mut writer = BufWriter::new(f);

    let mut u16_bytes: [u8; 2];
    let mut u32_bytes: [u8; 4];
    let mut u64_bytes: [u8; 8];

    // This will write a geotiff file in LittleEndian byte order regardless of the native endianness
    writer.write("II".as_bytes())?;

    u16_bytes = unsafe { mem::transmute(42u16.to_le()) };
    writer.write(&u16_bytes)?;

    // output the offset to the IFD
	let total_bytes_per_pixel = r.configs.data_type.get_data_size();
    if total_bytes_per_pixel == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "Unknown data type."));
    }

    let image_len = (r.configs.rows as usize * r.configs.columns as usize * total_bytes_per_pixel + 8usize) as u32;
    u32_bytes = unsafe { mem::transmute(image_len.to_le()) };
    writer.write(&u32_bytes)?;

    // output the data; compression is not currently supported for output
    match r.configs.photometric_interp {
        PhotometricInterpretation::Continuous |
        PhotometricInterpretation::Categorical |
        PhotometricInterpretation::Boolean => {
            match r.configs.data_type {
                DataType::F64 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u64_bytes = unsafe { mem::transmute(r.data[i]) };
                            writer.write(&u64_bytes)?;
                        }
                    }
                },
                DataType::F32 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u32_bytes = unsafe { mem::transmute(r.data[i] as f32) };
                            writer.write(&u32_bytes)?;
                        }
                    }
                },
                DataType::U64 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u64_bytes = unsafe { mem::transmute(r.data[i] as u64) };
                            writer.write(&u64_bytes)?;
                        }
                    }
                },
                DataType::U32 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u32_bytes = unsafe { mem::transmute(r.data[i] as u32) };
                            writer.write(&u32_bytes)?;
                        }
                    }
                },
                DataType::U16 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u16_bytes = unsafe { mem::transmute(r.data[i] as u16) };
                            writer.write(&u16_bytes)?;
                        }
                    }
                },
                DataType::U8 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            writer.write(&[r.data[i] as u8])?;
                        }
                    }
                },
                DataType::I64 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u64_bytes = unsafe { mem::transmute(r.data[i] as i64) };
                            writer.write(&u64_bytes)?;
                        }
                    }
                },
                DataType::I32 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u32_bytes = unsafe { mem::transmute(r.data[i] as i32) };
                            writer.write(&u32_bytes)?;
                        }
                    }
                },
                DataType::I16 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            u16_bytes = unsafe { mem::transmute(r.data[i] as i16) };
                            writer.write(&u16_bytes)?;
                        }
                    }
                },
                DataType::I8 => {
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            writer.write(&[r.data[i] as u8])?;
                        }
                    }
                },
                _ => { return Err(Error::new(ErrorKind::InvalidData, "Unknown data type.")); },
            }
        },
        PhotometricInterpretation::RGB => {
            match r.configs.data_type {
                DataType::RGB24 => {
                    let mut bytes: [u8; 3] = [0u8; 3];
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            let val = r.data[i] as u32;
                            bytes[0] = ((val >> 16u32) & 0xFF) as u8; // red
                            bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                            bytes[2] = (val & 0xFF) as u8; // blue
                            writer.write(&bytes)?;
                        }
                    }
                },
                DataType::RGBA32 => {
                    let mut bytes: [u8; 4] = [0u8; 4];
                    let mut i: usize;
                    for row in (0..r.configs.rows).rev() {
                        for col in 0..r.configs.columns {
                            i = row * r.configs.columns + col;
                            let val = r.data[i] as u32;
                            bytes[3] = ((val >> 24u32) & 0xFF) as u8; // alpha
                            bytes[0] = ((val >> 16u32) & 0xFF) as u8; // red
                            bytes[1] = ((val >> 8u32) & 0xFF) as u8; // green
                            bytes[2] = (val & 0xFF) as u8; // blue
                            writer.write(&bytes)?;
                        }
                    }
                },
                _ => { return Err(Error::new(ErrorKind::InvalidData, "Unknown data type.")); },
            }
        },
        PhotometricInterpretation::Paletted => {
            return Err(Error::new(ErrorKind::InvalidData, "Paletted GeoTIFFs are currently unsupported for writing."));
        },
        PhotometricInterpretation::Unknown => {
            return Err(Error::new(ErrorKind::InvalidData, "Error while writing GeoTIFF file."));
        },
    }

    // create the ifd's
    let mut ifd: Vec<IfdDirectory> = vec![];

    u32_bytes = unsafe { mem::transmute((r.configs.columns as u32).to_le()) };
    ifd.push(IfdDirectory::new( TAG_IMAGEWIDTH, DT_SHORT, 1u32, r.configs.columns as u32,
        u32_bytes.iter().cloned().collect(), Endianness::LittleEndian ));

    u32_bytes = unsafe { mem::transmute((r.configs.rows as u32).to_le()) };
    ifd.push(IfdDirectory::new( TAG_IMAGELENGTH, DT_SHORT, 1u32, r.configs.columns as u32,
        u32_bytes.iter().cloned().collect(), Endianness::LittleEndian ));

    let bps = match r.configs.photometric_interp {
        PhotometricInterpretation::RGB => {
            match r.configs.data_type {
                DataType::RGB24 => vec![8u16, 8u16, 8u16],
                DataType::RGBA32 => vec![8u16, 8u16, 8u16, 8u16],
                DataType::RGB48 => vec![16u16, 16u16, 16u16],
                _ => { return Err(Error::new(ErrorKind::InvalidData, "Unknown data type.")); }
            }
        },
        _ => {
            match r.configs.data_type {
                DataType::I8 | DataType::U8 => vec![8u16],
                DataType::I16 | DataType::U16 => vec![16u16],
                DataType::I32 | DataType::U32 | DataType::F32 => vec![32u16],
                DataType::I64 | DataType::U64 | DataType::F64 => vec![64u16],
                _ => { return Err(Error::new(ErrorKind::InvalidData, "Unknown data type.")); }
            }
        }
    };
    let _byte_size = bps.len() * 2 as usize;


    //let raw_bytes : [u8; byte_size] = unsafe {mem::transmute(bps)};
    // var bps = make([]uint16, g.samplesPerPixel)
    // for i := 0; i < int(g.samplesPerPixel); i++ {
    //     bps[i] = uint16(g.BitsPerSample[i])
    // }
    // ifd = append(ifd, CreateIfdEntry(tBitsPerSample, dtShort, uint32(g.samplesPerPixel), bps, g.ByteOrder))

    // fn new(tag: u16, ifd_type: u16, num_values: u32, offset: u32, data: Vec<u8>, byte_order: RasterByteOrder)
    ifd.push(IfdDirectory::new( TAG_COMPRESSION, DT_SHORT, 1u32, COMPRESS_NONE as u32,
        vec![1u8], Endianness::LittleEndian ));

    let out_pi = match r.configs.photometric_interp {
        PhotometricInterpretation::Continuous | PhotometricInterpretation::Categorical => PI_BLACKISZERO,
        PhotometricInterpretation::RGB => PI_RGB,
        PhotometricInterpretation::Paletted => PI_PALETTED,
        _ => { return Err(Error::new(ErrorKind::InvalidData, "Unknown data type.")); }
    };
    ifd.push(IfdDirectory::new( TAG_PHOTOMETRICINTERPRETATION, DT_SHORT, 1u32, out_pi as u32,
        vec![out_pi as u8], Endianness::LittleEndian ));

    //let mut strip_offsets: Vec<u32> = vec![];
    let total_bytes_per_pixel = match r.configs.data_type {
        DataType::I8 | DataType::U8 => 1u32,
        DataType::I16 | DataType::U16 => 2u32,
        DataType::I32 | DataType::U32 | DataType::F32 => 4u32,
        DataType::I64 | DataType::U64 | DataType::F64 => 8u32,
        DataType::RGB24 => 3u32,
        DataType::RGBA32 => 4u32,
        DataType::RGB48 => 6u32,
        _ => { return Err(Error::new(ErrorKind::InvalidData, "Unknown data type.")); }
    };
    let row_length_in_bytes: u32 = r.configs.columns as u32 * total_bytes_per_pixel;
    //let strip_byte_count: Vec<u32> = vec![row_length_in_bytes; r.configs.rows as usize];
    let mut strip_offsets_as_bytes: Vec<u8> = vec![];
    let mut strip_byte_count_as_bytes: Vec<u8> = vec![];
    for i in 0..r.configs.rows as u32 {
        let val = 8u32 + row_length_in_bytes * i;
        //strip_offsets.push(val);
        u32_bytes = unsafe { mem::transmute(val.to_le()) };
        strip_offsets_as_bytes.push(u32_bytes[0]);
        strip_offsets_as_bytes.push(u32_bytes[1]);
        strip_offsets_as_bytes.push(u32_bytes[2]);
        strip_offsets_as_bytes.push(u32_bytes[3]);

        u32_bytes = unsafe { mem::transmute(row_length_in_bytes.to_le()) };
        strip_byte_count_as_bytes.push(u32_bytes[0]);
        strip_byte_count_as_bytes.push(u32_bytes[1]);
        strip_byte_count_as_bytes.push(u32_bytes[2]);
        strip_byte_count_as_bytes.push(u32_bytes[3]);
    }

    ifd.push(IfdDirectory::new( TAG_STRIPOFFSETS, DT_LONG, r.configs.rows as u32, 0u32,
        strip_offsets_as_bytes, Endianness::LittleEndian ));

    let samples_per_pixel = match r.configs.data_type {
        DataType::RGB24 => 3u16,
        DataType::RGBA32 => 4u16,
        DataType::RGB48 => 3u16,
        _ => 1u16
    };
    u16_bytes = unsafe { mem::transmute(samples_per_pixel.to_le()) };
    ifd.push(IfdDirectory::new( TAG_SAMPLESPERPIXEL, DT_SHORT, 1u32, 0u32,
        u16_bytes.iter().cloned().collect(), Endianness::LittleEndian ));

    u16_bytes = unsafe { mem::transmute(1u16.to_le()) };
    ifd.push(IfdDirectory::new( TAG_ROWSPERSTRIP, DT_SHORT, 1u32, 1u32,
        u16_bytes.iter().cloned().collect(), Endianness::LittleEndian ));

    ifd.push(IfdDirectory::new( TAG_STRIPBYTECOUNTS, DT_LONG, r.configs.rows as u32, 0u32,
        strip_byte_count_as_bytes, Endianness::LittleEndian ));

    let software: String = String::from("whitebox-tools");
    let software_len = software.len() as u32;
    ifd.push(IfdDirectory::new( TAG_SOFTWARE, DT_ASCII, software_len, 0u32,
        software.into_bytes(), Endianness::LittleEndian ));

    // sf := make([]uint16, g.samplesPerPixel)
    // for i := 0; i < int(g.samplesPerPixel); i++ {
    //     sf[i] = uint16(g.SampleFormat)
    // }
    // ifd = append(ifd, CreateIfdEntry(tSampleFormat, dtShort, uint32(g.samplesPerPixel), sf, g.ByteOrder))
    //
    // if g.samplesPerPixel > 1 {
    //     ifd = append(ifd, CreateIfdEntry(tPlanarConfiguration, dtShort, 1, uint16(1), g.ByteOrder))
    // }
    //
    // if g.PhotometricInterp == PI_RGB && g.samplesPerPixel == 4 {
    //     ifd = append(ifd, CreateIfdEntry(tExtraSamples, dtShort, 1, uint16(1), g.ByteOrder))
    // }
    //
    // // There is currently no support for storing the image
    // // resolution, so give a bogus value of 72x72 dpi.
    // ifd = append(ifd, CreateIfdEntry(tXResolution, dtRational, 1, []uint32{72, 1}, g.ByteOrder))
    // ifd = append(ifd, CreateIfdEntry(tYResolution, dtRational, 1, []uint32{72, 1}, g.ByteOrder))
    // ifd = append(ifd, CreateIfdEntry(tResolutionUnit, dtShort, 1, []uint32{resPerInch}, g.ByteOrder))
    //
    // // Add the ModelPixelScaleTag and ModelTiepointTag tags
    // ifd = append(ifd, CreateIfdEntry(tModelTiepointTag, dtDouble, 6, g.TiepointData.getModelTiepointTagData(), g.ByteOrder))
    // ifd = append(ifd, CreateIfdEntry(tModelPixelScaleTag, dtDouble, 3, g.TiepointData.getModelPixelScaleTagData(), g.ByteOrder))
    //
    // if g.NodataValue != "" {
    //     ifd = append(ifd, CreateIfdEntry(tGDAL_NODATA, dtASCII, uint32(len(g.NodataValue)), g.NodataValue, g.ByteOrder))
    // }


    let _ = writer.flush();

    Ok(())
}

#[derive(Default, Clone, Debug)]
pub struct IfdDirectory {
    pub tag: u16,
    pub ifd_type: u16,
    pub num_values: u32,
    pub offset: u32,
    pub data: Vec<u8>,
    byte_order: Endianness
}

impl IfdDirectory {
    pub fn new(tag: u16, ifd_type: u16, num_values: u32, offset: u32, data: Vec<u8>, byte_order: Endianness) -> IfdDirectory {
        IfdDirectory { tag: tag, ifd_type: ifd_type, num_values: num_values, offset: offset, data: data, byte_order: byte_order}
    }

    pub fn interpret_as_u16(&self) -> Vec<u16> {
        let mut bor = ByteOrderReader::new(self.data.clone(), self.byte_order);
        let mut vals: Vec<u16> = vec![];
        for _ in 0..self.num_values {
            let val = bor.read_u16();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_u32(&self) -> Vec<u32> {
        let mut bor = ByteOrderReader::new(self.data.clone(), self.byte_order);
        let mut vals: Vec<u32> = vec![];
        for _ in 0..self.num_values {
            let val = bor.read_u32();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_f64(&self) -> Vec<f64> {
        let mut bor = ByteOrderReader::new(self.data.clone(), self.byte_order);
        let mut vals: Vec<f64> = vec![];
        for _ in 0..self.num_values {
            let val = bor.read_f64();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_ascii(&self) -> String {
        if self.data[self.data.len()-1] == 0 {
            let s = &self.data[0..self.data.len()-1];
            return String::from_utf8(s.to_vec()).unwrap();
        } else {
            return String::from_utf8(self.data.clone()).unwrap();
        }

    }

    pub fn interpret_data(&self) -> String {
        let mut bor = ByteOrderReader::new(self.data.clone(), self.byte_order);
        if self.ifd_type == 2 { // ascii
            return String::from_utf8(self.data.clone()).unwrap();
        } else if self.ifd_type == 3 { // u16
            let mut vals: Vec<u16> = vec![];
            for _ in 0..self.num_values {
                let val = bor.read_u16();
                vals.push(val);
            }
            if self.num_values == 1 {
                let kw_map = get_keyword_map();
                let map = match kw_map.get(&self.tag) {
                    Some(map) => map,
                    None => return format!("{:?}", vals),
                };
                match map.get(&vals[0]) {
                    Some(v) => return format!("{:?} ({})", v, vals[0]),
                    None => return format!("{:?}", vals),
                }
            } else {
                return format!("{:?}", vals);
            }
        } else if self.ifd_type == 4 { // u32
            let mut vals: Vec<u32> = vec![];
            for _ in 0..self.num_values {
                let val = bor.read_u32();
                vals.push(val);
            }
            return format!("{:?}", vals);
        } else if self.ifd_type == 12 { // f64
            let mut vals: Vec<f64> = vec![];
            for _ in 0..self.num_values {
                let val = bor.read_f64();
                vals.push(val);
            }
            return format!("{:?}", vals);
        } else {
            return format!("{:?}", self.data);
        }
    }
}

impl fmt::Display for IfdDirectory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag_map = get_keys_map();
        //let kw_map = get_keyword_map();
        let ft_map = get_field_type_map();

        let mut s = format!("\nTag {}", tag_map[&self.tag]);
        s = s + &format!("\nIFD_type: {} ({})", ft_map[&self.ifd_type], self.ifd_type);
        s = s + &format!("\nNum_values: {}", self.num_values);
        s = s + &format!("\nOffset: {}", self.offset);
        s = s + &format!("\nData: {}", self.interpret_data());
        write!(f, "{}", s)
    }
}

pub enum ImageMode {
    Bilevel = 1,
    Paletted = 2,
    Gray = 3,
    GrayInvert = 4,
    RGB = 5,
    RGBA = 6,
    NRGBA = 7,
}

const COMPRESS_NONE: u16       = 1;
const COMPRESS_CCITT: u16      = 2;
const COMPRESS_G3: u16         = 3; // Group 3 Fax.
const COMPRESS_G4: u16         = 4; // Group 4 Fax.
const COMPRESS_LZW: u16        = 5;
const COMPRESS_JPEGOLD: u16    = 6; // Superseded by cJPEG.
const COMPRESS_JPEG: u16       = 7;
const COMPRESS_DEFLATE: u16    = 8; // zlib compression.
const COMPRESS_PACKBITS: u16   = 32773;
const COMPRESS_DEFLATEOLD: u16 = 32946; // Superseded by cDeflate.

const DT_BYTE: u16      = 1;
const DT_ASCII: u16     = 2;
const DT_SHORT: u16     = 3;
const DT_LONG: u16      = 4;
const DT_RATIONAL: u16  = 5;
const DT_SBYTE: u16     = 6;
const DT_UNDEFINED: u16 = 7;
const DT_SSHORT: u16    = 8;
const DT_SLONG: u16     = 9;
const DT_SRATIONAL: u16 = 10;
const DT_FLOAT: u16     = 11;
const DT_DOUBLE: u16    = 12;

const PI_WHITEISZERO: u16 = 0;
const PI_BLACKISZERO: u16 = 1;
const PI_RGB: u16         = 2;
const PI_PALETTED: u16    = 3;
// const PI_TRANSMASK: u16   = 4; // transparency mask
// const PI_CMYK: u16        = 5;
// const PI_YCBCR: u16       = 6;
// const PI_CIELAB: u16      = 8;

// Tags (see p. 28-41 of the spec).
const TAG_NEWSUBFILETYPE: u16            = 254u16;
const TAG_IMAGEWIDTH: u16                = 256u16;
const TAG_IMAGELENGTH: u16               = 257u16;
const TAG_BITSPERSAMPLE: u16             = 258u16;
const TAG_COMPRESSION: u16               = 259u16;
const TAG_PHOTOMETRICINTERPRETATION: u16 = 262u16;
const TAG_FILLORDER: u16                 = 266u16;
const TAG_DOCUMENTNAME: u16              = 269u16;
const TAG_PLANARCONFIGURATION: u16       = 284u16;

const TAG_STRIPOFFSETS: u16    = 273u16;
const TAG_ORIENTATION: u16     = 274u16;
const TAG_SAMPLESPERPIXEL: u16 = 277u16;
const TAG_ROWSPERSTRIP: u16    = 278u16;
const TAG_STRIPBYTECOUNTS: u16 = 279u16;

const TAG_TILEWIDTH: u16      = 322u16;
const TAG_TILELENGTH: u16     = 323u16;
const TAG_TILEOFFSETS: u16    = 324u16;
const TAG_TILEBYTECOUNTS: u16 = 325u16;

const TAG_XRESOLUTION: u16    = 282u16;
const TAG_YRESOLUTION: u16    = 283u16;
const TAG_RESOLUTIONUNIT: u16 = 296u16;

const TAG_SOFTWARE: u16     = 305u16;
const TAG_PREDICTOR: u16    = 317u16;
const TAG_COLORMAP: u16     = 320u16;
const TAG_EXTRASAMPLES: u16 = 338u16;
const TAG_SAMPLEFORMAT: u16 = 339u16;

const TAG_GDAL_METADATA: u16 = 42112u16;
const TAG_GDAL_NODATA: u16   = 42113u16;

const TAG_MODELPIXELSCALETAG: u16     = 33550u16;
const TAG_MODELTRANSFORMATIONTAG: u16 = 34264u16;
const TAG_MODELTIEPOINTTAG: u16       = 33922u16;
const TAG_GEOKEYDIRECTORYTAG: u16     = 34735u16;
const TAG_GEODOUBLEPARAMSTAG: u16     = 34736u16;
const TAG_GEOASCIIPARAMSTAG: u16      = 34737u16;
const TAG_INTERGRAPHMATRIXTAG: u16    = 33920u16;

const TAG_GTMODELTYPEGEOKEY: u16              = 1024u16;
const TAG_GTRASTERTYPEGEOKEY: u16             = 1025u16;
const TAG_GTCITATIONGEOKEY: u16               = 1026u16;
const TAG_GEOGRAPHICTYPEGEOKEY: u16           = 2048u16;
const TAG_GEOGCITATIONGEOKEY: u16             = 2049u16;
const TAG_GEOGGEODETICDATUMGEOKEY: u16        = 2050u16;
const TAG_GEOGPRIMEMERIDIANGEOKEY: u16        = 2051u16;
const TAG_GEOGLINEARUNITSGEOKEY: u16          = 2052u16;
const TAG_GEOGLINEARUNITSIZEGEOKEY: u16       = 2053u16;
const TAG_GEOGANGULARUNITSGEOKEY: u16         = 2054u16;
const TAG_GEOGANGULARUNITSIZEGEOKEY: u16      = 2055u16;
const TAG_GEOGELLIPSOIDGEOKEY: u16            = 2056u16;
const TAG_GEOGSEMIMAJORAXISGEOKEY: u16        = 2057u16;
const TAG_GEOGSEMIMINORAXISGEOKEY: u16        = 2058u16;
const TAG_GEOGINVFLATTENINGGEOKEY: u16        = 2059u16;
const TAG_GEOGAZIMUTHUNITSGEOKEY: u16         = 2060u16;
const TAG_GEOGPRIMEMERIDIANLONGGEOKEY: u16    = 2061u16;
const TAG_PROJECTEDCSTYPEGEOKEY: u16          = 3072u16;
const TAG_PCSCITATIONGEOKEY: u16              = 3073u16;
const TAG_PROJECTIONGEOKEY: u16               = 3074u16;
const TAG_PROJCOORDTRANSGEOKEY: u16           = 3075u16;
const TAG_PROJLINEARUNITSGEOKEY: u16          = 3076u16;
const TAG_PROJLINEARUNITSIZEGEOKEY: u16       = 3077u16;
const TAG_PROJSTDPARALLEL1GEOKEY: u16         = 3078u16;
const TAG_PROJSTDPARALLEL2GEOKEY: u16         = 3079u16;
const TAG_PROJNATORIGINLONGGEOKEY: u16        = 3080u16;
const TAG_PROJNATORIGINLATGEOKEY: u16         = 3081u16;
const TAG_PROJFALSEEASTINGGEOKEY: u16         = 3082u16;
const TAG_PROJFALSENORTHINGGEOKEY: u16        = 3083u16;
const TAG_PROJFALSEORIGINLONGGEOKEY: u16      = 3084u16;
const TAG_PROJFALSEORIGINLATGEOKEY: u16       = 3085u16;
const TAG_PROJFALSEORIGINEASTINGGEOKEY: u16   = 3086u16;
const TAG_PROJFALSEORIGINNORTHINGGEOKEY: u16  = 3087u16;
const TAG_PROJCENTERLONGGEOKEY: u16           = 3088u16;
const TAG_PROJCENTERLATGEOKEY: u16            = 3089u16;
const TAG_PROJCENTEREASTINGGEOKEY: u16        = 3090u16;
const TAG_PROJCENTERNORTHINGGEOKEY: u16       = 3091u16;
const TAG_PROJSCALEATNATORIGINGEOKEY: u16     = 3092u16;
const TAG_PROJSCALEATCENTERGEOKEY: u16        = 3093u16;
const TAG_PROJAZIMUTHANGLEGEOKEY: u16         = 3094u16;
const TAG_PROJSTRAIGHTVERTPOLELONGGEOKEY: u16 = 3095u16;
const TAG_VERTICALCSTYPEGEOKEY: u16           = 4096u16;
const TAG_VERTICALCITATIONGEOKEY: u16         = 4097u16;
const TAG_VERTICALDATUMGEOKEY: u16            = 4098u16;
const TAG_VERTICALUNITSGEOKEY: u16            = 4099u16;

const TAG_PHOTOSHOP: u16 = 34377u16;

// An implimentation of a PackBits reader
#[inline]
pub fn packbits_decoder(input_data: Vec<u8>) -> Vec<u8> {
    let mut output_data = vec![];
    let mut i: usize = 0;
    while i < input_data.len() {
        let hex = input_data[i];
        if hex >= 128 {
            let hex2 = (256i16 - hex as i16) as u8;
            for _ in 0..(hex2+1) {
                output_data.push(input_data[i + 1]);
            }
            i += 1;
        } else {
            for j in 0..(hex+1) {
                output_data.push(input_data[i + j as usize + 1]);
            }
            i += hex as usize + 1;
        }
        i += 1;
    }
    output_data
}
