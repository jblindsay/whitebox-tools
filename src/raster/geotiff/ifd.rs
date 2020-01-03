use super::geokeys;
use crate::utils::{ByteOrderReader, Endianness};
use std::fmt;
use std::io::Cursor;

#[derive(Default, Clone, Debug)]
pub struct Ifd {
    pub tag: u16,
    pub ifd_type: u16,
    pub num_values: u64,
    pub offset: u64,
    pub data: Vec<u8>,
    byte_order: Endianness,
}

impl Ifd {
    pub fn new(
        tag: u16,
        ifd_type: u16,
        num_values: u64,
        offset: u64,
        data: Vec<u8>,
        byte_order: Endianness,
    ) -> Ifd {
        Ifd {
            tag: tag,
            ifd_type: ifd_type,
            num_values: num_values,
            offset: offset,
            data: data,
            byte_order: byte_order,
        }
    }

    pub fn interpret_as_u16(&self) -> Vec<u16> {
        let mut bor = ByteOrderReader::<Cursor<Vec<u8>>>::new(
            Cursor::new(self.data.clone()),
            self.byte_order,
        );
        let mut vals: Vec<u16> = vec![];
        let mut val: u16;
        for _ in 0..self.num_values {
            val = bor.read_u16().unwrap();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_u32(&self) -> Vec<u32> {
        let mut bor = ByteOrderReader::<Cursor<Vec<u8>>>::new(
            Cursor::new(self.data.clone()),
            self.byte_order,
        );
        let mut vals: Vec<u32> = vec![];
        let mut val: u32;
        for _ in 0..self.num_values {
            val = bor.read_u32().unwrap();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_u64(&self) -> Vec<u64> {
        let mut bor = ByteOrderReader::<Cursor<Vec<u8>>>::new(
            Cursor::new(self.data.clone()),
            self.byte_order,
        );
        let mut vals: Vec<u64> = vec![];
        let mut val: u64;
        for _ in 0..self.num_values {
            val = bor.read_u64().unwrap();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_i64(&self) -> Vec<i64> {
        let mut bor = ByteOrderReader::<Cursor<Vec<u8>>>::new(
            Cursor::new(self.data.clone()),
            self.byte_order,
        );
        let mut vals: Vec<i64> = vec![];
        let mut val: i64;
        for _ in 0..self.num_values {
            val = bor.read_i64().unwrap();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_f64(&self) -> Vec<f64> {
        let mut bor = ByteOrderReader::<Cursor<Vec<u8>>>::new(
            Cursor::new(self.data.clone()),
            self.byte_order,
        );
        let mut vals: Vec<f64> = vec![];
        let mut val: f64;
        for _ in 0..self.num_values {
            val = bor.read_f64().unwrap();
            vals.push(val);
        }
        vals
    }

    pub fn interpret_as_ascii(&self) -> String {
        let mut num_trailing_zeros = 0;
        for d in self.data.iter().rev() {
            if *d == 0u8 {
                num_trailing_zeros += 1;
            }
        }
        let s = &self.data[0..(self.data.len() - num_trailing_zeros)];
        let ret = match String::from_utf8(s.to_vec()) {
            Ok(v) => v,
            Err(e) => panic!(
                "Error converting TAG({}) to ASCII (value={:?}) {}",
                self.tag,
                self.data.clone(),
                e
            ),
        };
        return ret.trim().to_owned();

        // if self.data[self.data.len() - 1] == 0 {
        //     let s = &self.data[0..self.data.len() - 1];
        //     let ret = match String::from_utf8(s.to_vec()) {
        //         Ok(v) => v,
        //         Err(e) => panic!("Error converting TAG({}) to ASCII (value={:?}) {}", self.tag, self.data.clone(), e),
        //     };
        //     // String::from_utf8(s.to_vec()).unwrap();
        //     return ret
        // } else {
        //     let ret = match String::from_utf8(self.data.clone()) {
        //         Ok(v) => v,
        //         Err(e) => panic!("Error converting TAG({}) to ASCII (value={:?}) {}", self.tag, self.data.clone(), e),
        //     };
        //     // String::from_utf8(self.data.clone()).unwrap();
        //     return ret
        // }
    }

    pub fn interpret_data(&self) -> String {
        // sanity check: don't print out thousands of values in a tag.
        let how_many_vals = if self.num_values < 100 {
            self.num_values
        } else {
            100u64
        };
        let mut bor = ByteOrderReader::<Cursor<Vec<u8>>>::new(
            Cursor::new(self.data.clone()),
            self.byte_order,
        );
        if self.ifd_type == 2 {
            // ascii
            return String::from_utf8(self.data.clone()).unwrap();
        } else if self.ifd_type == 3 {
            // u16
            let mut vals: Vec<u16> = vec![];
            for _ in 0..how_many_vals {
                let val = bor.read_u16().unwrap();
                vals.push(val);
            }
            if self.num_values == 1 {
                let kw_map = geokeys::get_keyword_map();
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
        } else if self.ifd_type == 4 {
            // u32
            let mut vals: Vec<u32> = vec![];
            for _ in 0..how_many_vals {
                let val = bor.read_u32().unwrap();
                vals.push(val);
            }
            return format!("{:?}", vals);
        } else if self.ifd_type == 12 {
            // f64
            let mut vals: Vec<f64> = vec![];
            for _ in 0..how_many_vals {
                let val = bor.read_f64().unwrap();
                vals.push(val);
            }
            return format!("{:?}", vals);
        } else if self.ifd_type == 16 || self.ifd_type == 18 {
            // u64
            let mut vals: Vec<u64> = vec![];
            for _ in 0..how_many_vals {
                let val = bor.read_u64().unwrap();
                vals.push(val);
            }
            return format!("{:?}", vals);
        } else if self.ifd_type == 17 {
            // u64
            let mut vals: Vec<i64> = vec![];
            let mut val: i64;
            for _ in 0..how_many_vals {
                val = bor.read_i64().unwrap();
                vals.push(val);
            }
            return format!("{:?}", vals);
        } else {
            return format!("{:?}", self.data);
        }
    }
}

impl fmt::Display for Ifd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag_map = geokeys::get_keys_map();
        //let kw_map = get_keyword_map();
        let ft_map = geokeys::get_field_type_map();

        if tag_map.get(&self.tag).is_some() {
            let off = if self.num_values > 1 || self.ifd_type > 3 {
                format!(" offset={}", self.offset)
            } else {
                String::from("")
            };

            let mut d = if self.ifd_type != 2 {
                format!("{}", self.interpret_data())
            } else {
                format!("{}", self.interpret_data().replace("\0", ""))
            };

            let c = if self.num_values > 1 {
                format!(" count={}", self.num_values)
            } else {
                d = d.replace("[", "").replace("]", "");
                String::from("")
            };

            let s = format!(
                "{} (code={} type={}{}{}): {}",
                tag_map[&self.tag].name, tag_map[&self.tag].code, ft_map[&self.ifd_type], c, off, d
            );

            return write!(f, "{}", s);
        }

        let mut s = format!("\nUnrecognized Tag ({})", &self.tag);
        s = s + &format!("\nIFD_type: {} ({})", ft_map[&self.ifd_type], self.ifd_type);
        s = s + &format!("\nNum_values: {}", self.num_values);
        if self.num_values > 1 || self.ifd_type > 3 {
            s = s + &format!("\nOffset: {}", self.offset);
        }
        if self.ifd_type != 2 {
            s = s + &format!("\nData: {}", self.interpret_data());
        } else {
            s = s + &format!("\nData: {}", self.interpret_data().replace("\0", ""));
        }
        write!(f, "{}", s)
    }
}

#[derive(Default, Clone, Debug)]
pub(super) struct IfdEntry {
    pub tag: u16,
    pub ifd_type: u16,
    pub num_values: u32,
    pub offset: u32,
}

impl IfdEntry {
    pub(super) fn new(tag: u16, ifd_type: u16, num_values: u32, offset: u32) -> IfdEntry {
        IfdEntry {
            tag,
            ifd_type,
            num_values,
            offset,
        }
    }
}

impl fmt::Display for IfdEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag_map = geokeys::get_keys_map();
        let ft_map = geokeys::get_field_type_map();

        let mut s = format!("\nTag {} {}", &self.tag, tag_map[&self.tag]);
        s = s + &format!("\nIFD_type: {} ({})", ft_map[&self.ifd_type], self.ifd_type);
        s = s + &format!("\nNum_values: {}", self.num_values);
        s = s + &format!("\nOffset: {}", self.offset);
        write!(f, "{}", s)
    }
}

#[derive(Default, Clone, Debug)]
pub(super) struct Entry {
    pub tag: u16,
    pub ifd_type: u16,
    pub num_values: u64,
    pub offset: u64,
}

impl Entry {
    pub(super) fn new(tag: u16, ifd_type: u16, num_values: u64, offset: u64) -> Entry {
        Entry {
            tag,
            ifd_type,
            num_values,
            offset,
        }
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag_map = geokeys::get_keys_map();
        let ft_map = geokeys::get_field_type_map();

        let mut s = format!("\nTag {} {}", &self.tag, tag_map[&self.tag]);
        s = s + &format!("\nIFD_type: {} ({})", ft_map[&self.ifd_type], self.ifd_type);
        s = s + &format!("\nNum_values: {}", self.num_values);
        s = s + &format!("\nOffset: {}", self.offset);
        write!(f, "{}", s)
    }
}
