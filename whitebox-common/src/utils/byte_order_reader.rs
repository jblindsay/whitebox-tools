/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: Unknown
Last Modified: 22/10/2019
License: MIT
*/
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::prelude::*;
use std::io::{Result, SeekFrom};

pub struct ByteOrderReader<R: Read + Seek> {
    is_le: bool,
    reader: R,
    pos: usize,
    len: usize,
}

impl<R: Read + Seek> ByteOrderReader<R> {
    pub fn new(reader: R, byte_order: Endianness) -> ByteOrderReader<R> {
        let is_le = byte_order == Endianness::LittleEndian;
        let mut bor = ByteOrderReader {
            reader: reader,
            is_le: is_le,
            pos: 0usize,
            len: 0, // don't know the length yet
        };
        // now get the length
        let len = bor.reader.seek(SeekFrom::End(0)).unwrap() as usize;
        bor.len = len;
        bor.seek(0); // return the cursor to the start.
        bor
    }

    pub fn set_byte_order(&mut self, byte_order: Endianness) {
        self.is_le = byte_order == Endianness::LittleEndian;
    }

    pub fn get_byte_order(&self) -> Endianness {
        if self.is_le {
            return Endianness::LittleEndian;
        }
        Endianness::BigEndian
    }

    pub fn seek(&mut self, position: usize) {
        self.pos = position;
        self.reader.seek(SeekFrom::Start(self.pos as u64)).unwrap();
    }

    pub fn inc_pos(&mut self, skip: usize) {
        self.pos += skip;
        self.reader.seek(SeekFrom::Start(self.pos as u64)).unwrap();
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn len(&self) -> usize {
        self.len
    }
 
    pub fn is_empty(&self) -> bool {
        if self.len() > 0 {
            return true;
        }
        false
    }

    pub fn read_utf8(&mut self, length: usize) -> String {
        let mut bytes = vec![0u8; length];
        self.reader.read_exact(&mut bytes).unwrap();
        let val = String::from_utf8_lossy(&bytes).to_string();
        self.pos += length;
        val
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.pos += 1;
        self.reader.read_u8()
    }

    pub fn peek_u8(&mut self) -> Result<u8> {
        let val = self.reader.read_u8();
        self.seek(self.pos);
        val
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.pos += buf.len();
        self.reader.read_exact(buf)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        self.pos += 2;
        if self.is_le {
            return self.reader.read_u16::<LittleEndian>();
        }
        self.reader.read_u16::<BigEndian>()
    }

    pub fn read_u24(&mut self) -> Result<u32> {
        self.pos += 3;
        if self.is_le {
            return self.reader.read_u24::<LittleEndian>();
        }
        self.reader.read_u24::<BigEndian>()
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        self.pos += 4;
        if self.is_le {
            return self.reader.read_u32::<LittleEndian>();
        }
        self.reader.read_u32::<BigEndian>()
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        self.pos += 8;
        if self.is_le {
            return self.reader.read_u64::<LittleEndian>();
        }
        self.reader.read_u64::<BigEndian>()
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        // There's really no need for endian issues when reading single bytes.
        self.pos += 1;
        self.reader.read_i8()
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        self.pos += 2;
        if self.is_le {
            return self.reader.read_i16::<LittleEndian>();
        }
        self.reader.read_i16::<BigEndian>()
    }

    pub fn read_i24(&mut self) -> Result<i32> {
        self.pos += 3;
        if self.is_le {
            return self.reader.read_i24::<LittleEndian>();
        }
        self.reader.read_i24::<BigEndian>()
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        self.pos += 4;
        if self.is_le {
            return self.reader.read_i32::<LittleEndian>();
        }
        self.reader.read_i32::<BigEndian>()
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        self.pos += 8;
        if self.is_le {
            return self.reader.read_i64::<LittleEndian>();
        }
        self.reader.read_i64::<BigEndian>()
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        self.pos += 4;
        if self.is_le {
            return self.reader.read_f32::<LittleEndian>();
        }
        self.reader.read_f32::<BigEndian>()
    }

    // pub fn as_f32_vec(&mut self) -> Vec<f32> {
    //     let num_values = self.reader.len() / 4;
    //     let mut ret: Vec<f32> = Vec::with_capacity(num_values);
    //     if self.is_le {
    //         for a in 0..num_values {
    //             ret.push(LittleEndian::read_f32(&self.reader[a*4..a*4+4]));
    //         }
    //     } else {
    //         for a in 0..num_values {
    //             ret.push(BigEndian::read_f32(&self.reader[a*4..a*4+4]));
    //         }
    //     }
    //     ret
    // }

    pub fn read_f64(&mut self) -> Result<f64> {
        self.pos += 8;
        if self.is_le {
            return self.reader.read_f64::<LittleEndian>();
        }
        self.reader.read_f64::<BigEndian>()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Endianness {
    LittleEndian,
    BigEndian,
}

impl Default for Endianness {
    fn default() -> Endianness {
        Endianness::LittleEndian
    }
}

impl Endianness {
    pub fn from_str(val: &str) -> Endianness {
        let val_lc: &str = &val.to_lowercase();
        if val_lc.contains("lsb")
            || val_lc.contains("little")
            || val_lc.contains("intel")
            || val_lc.contains("least")
        {
            return Endianness::LittleEndian;
        }
        
        Endianness::BigEndian
    }
}
