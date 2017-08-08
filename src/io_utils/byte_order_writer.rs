// use std::io::prelude::*;
// use std::io::Error;
// use std::io::BufWriter;
// use std::fs::File;
// use byteorder::{ByteOrder, LittleEndian, BigEndian, WriteBytesExt};
// use io_utils::byte_order_reader::Endianness;

// pub struct ByteOrderWriter {
//     pub byte_order: Endianness,
//     pub writer: &mut BufWriter<File>,
// }

// impl ByteOrderWriter {
//     pub fn new<'a>(buffer: &'a mut BufWriter<File>, byte_order: Endianness) -> ByteOrderWriter {
//         ByteOrderWriter {
//             writer: buffer,
//             byte_order: byte_order,
//         }
//     }

//     pub fn write_u8(&mut self, value: u8) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_u8::<LittleEndian>(value)
//         } else {
//             self.writer.write_u8::<BigEndian>(value)
//         }
//     }

//     pub fn write_u16(&mut self, value: u16) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             // LittleEndian::write_u16(&mut self.writer, value)
//             self.writer.write_u16::<LittleEndian>(value)
//         } else {
//             // BigEndian::write_u16(&mut self.writer, value)
//             self.writer.write_u16::<BigEndian>(value)
//         }
//     }

//     pub fn write_u32(&mut self, value: u32) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_u32::<LittleEndian>(value)
//         } else {
//             self.writer.write_u32::<BigEndian>(value)
//         }
//     }

//     pub fn write_u64(&mut self, value: u64) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_u64::<LittleEndian>(value)
//         } else {
//             self.writer.write_u64::<BigEndian>(value)
//         }
//     }

//     pub fn write_i8(&mut self, value: i8) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_i8::<LittleEndian>(value)
//         } else {
//             self.writer.write_i8::<BigEndian>(value)
//         }
//     }

//     pub fn write_i16(&mut self, value: i16) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_i16::<LittleEndian>(value)
//         } else {
//             self.writer.write_i16::<BigEndian>(value)
//         }
//     }

//     pub fn write_i32(&mut self, value: i32) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_i32::<LittleEndian>(value)
//         } else {
//             self.writer.write_i32::<BigEndian>(value)
//         }
//     }

//     pub fn write_i64(&mut self, value: i64) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_i64::<LittleEndian>(value)
//         } else {
//             self.writer.write_i64::<BigEndian>(value)
//         }
//     }

//     pub fn write_f32(&mut self, value: f32) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_f32::<LittleEndian>(value)
//         } else {
//             self.writer.write_f32::<BigEndian>(value)
//         }
//     }

//     pub fn write_f64(&mut self, value: f64) -> Result<(), Error> {
//         if self.byte_order == Endianness::LittleEndian {
//             self.writer.write_f64::<LittleEndian>(value)
//         } else {
//             self.writer.write_f64::<BigEndian>(value)
//         }
//     }
// }
