use super::byte_order_reader::Endianness;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::io::prelude::*;
use std::io::Error;

pub struct ByteOrderWriter<W: Write> {
    is_le: bool,
    writer: W,
    num_bytes_written: usize,
}

impl<W: Write> ByteOrderWriter<W> {
    pub fn new(writer: W, byte_order: Endianness) -> ByteOrderWriter<W> {
        let is_le = byte_order == Endianness::LittleEndian;
        ByteOrderWriter::<W> {
            writer: writer,
            is_le: is_le,
            num_bytes_written: 0,
        }
    }

    // pub fn seek_from_start(&mut self, loc: u64) {
    //     if loc < self.num_bytes_written as u64 {
    //         let _ = self.writer.seek(SeekFrom::Start(loc));
    //     }
    // }

    // pub fn seek_end(&mut self) {
    //     let _ = self.writer.seek(SeekFrom::End(0));
    // }

    pub fn get_num_bytes_written(&self) -> usize {
        self.num_bytes_written
    }

    pub fn set_byte_order(&mut self, byte_order: Endianness) {
        self.is_le = byte_order == Endianness::LittleEndian;
    }

    pub fn write_u8(&mut self, value: u8) -> Result<(), Error> {
        self.num_bytes_written += 1;
        self.writer.write_u8(value)
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.num_bytes_written += bytes.len();
        self.writer.write_all(bytes)
    }

    pub fn write_u16(&mut self, value: u16) -> Result<(), Error> {
        self.num_bytes_written += 2;
        if self.is_le {
            self.writer.write_u16::<LittleEndian>(value)
        } else {
            self.writer.write_u16::<BigEndian>(value)
        }
    }

    pub fn write_u32(&mut self, value: u32) -> Result<(), Error> {
        self.num_bytes_written += 4;
        if self.is_le {
            self.writer.write_u32::<LittleEndian>(value)
        } else {
            self.writer.write_u32::<BigEndian>(value)
        }
    }

    pub fn write_u64(&mut self, value: u64) -> Result<(), Error> {
        self.num_bytes_written += 8;
        if self.is_le {
            self.writer.write_u64::<LittleEndian>(value)
        } else {
            self.writer.write_u64::<BigEndian>(value)
        }
    }

    pub fn write_i8(&mut self, value: i8) -> Result<(), Error> {
        self.num_bytes_written += 1;
        self.writer.write_i8(value)
    }

    pub fn write_i16(&mut self, value: i16) -> Result<(), Error> {
        self.num_bytes_written += 2;
        if self.is_le {
            self.writer.write_i16::<LittleEndian>(value)
        } else {
            self.writer.write_i16::<BigEndian>(value)
        }
    }

    pub fn write_i32(&mut self, value: i32) -> Result<(), Error> {
        self.num_bytes_written += 4;
        if self.is_le {
            self.writer.write_i32::<LittleEndian>(value)
        } else {
            self.writer.write_i32::<BigEndian>(value)
        }
    }

    pub fn write_i64(&mut self, value: i64) -> Result<(), Error> {
        self.num_bytes_written += 8;
        if self.is_le {
            self.writer.write_i64::<LittleEndian>(value)
        } else {
            self.writer.write_i64::<BigEndian>(value)
        }
    }

    pub fn write_f32(&mut self, value: f32) -> Result<(), Error> {
        self.num_bytes_written += 4;
        if self.is_le {
            self.writer.write_f32::<LittleEndian>(value)
        } else {
            self.writer.write_f32::<BigEndian>(value)
        }
    }

    pub fn write_f64(&mut self, value: f64) -> Result<(), Error> {
        self.num_bytes_written += 8;
        if self.is_le {
            self.writer.write_f64::<LittleEndian>(value)
        } else {
            self.writer.write_f64::<BigEndian>(value)
        }
    }

    /// Returns the number of bytes written
    pub fn len(&mut self) -> usize {
        self.num_bytes_written
        // // self.writer.stream_len().unwrap() as usize
        // self.writer.seek(SeekFrom::End(0)).unwrap() as usize + 1
    }

    pub fn get_inner(&mut self) -> &W {
        &self.writer
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}
