use byteorder::{BigEndian, ByteOrder, LittleEndian};

pub struct ByteOrderReader {
    pub byte_order: Endianness,
    pub is_le: bool,
    pub buffer: Vec<u8>,
    pub pos: usize,
}

impl ByteOrderReader {
    pub fn new(buffer: Vec<u8>, byte_order: Endianness) -> ByteOrderReader {
        let is_le = byte_order == Endianness::LittleEndian;
        ByteOrderReader {
            buffer: buffer,
            byte_order: byte_order,
            is_le: is_le,
            pos: 0usize,
        }
    }

    pub fn seek(&mut self, position: usize) {
        self.pos = position;
    }

    pub fn inc_pos(&mut self, skip: usize) {
        self.pos += skip;
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn len(&mut self) -> usize {
        self.buffer.len()
    }

    pub fn read_utf8(&mut self, length: usize) -> String {
        let val = String::from_utf8_lossy(&self.buffer[self.pos..self.pos + length]).to_string();
        self.pos += length;
        val
    }

    pub fn read_u8(&mut self) -> u8 {
        // There's really no need for endian issues when reading single bytes.
        let val = self.buffer[self.pos];
        self.pos += 1;
        val
    }

    pub fn peek_u8(&mut self) -> u8 {
        let val = self.buffer[self.pos];
        val
    }

    pub fn read_u16(&mut self) -> u16 {
        let buf = &self.buffer[self.pos..self.pos + 2];
        self.pos += 2;
        if self.byte_order == Endianness::LittleEndian {
            LittleEndian::read_u16(buf)
        } else {
            BigEndian::read_u16(buf)
        }
    }

    pub fn read_u32(&mut self) -> u32 {
        let buf = &self.buffer[self.pos..self.pos + 4];
        self.pos += 4;
        if self.byte_order == Endianness::LittleEndian {
            LittleEndian::read_u32(buf)
        } else {
            BigEndian::read_u32(buf)
        }
    }

    pub fn read_u64(&mut self) -> u64 {
        let buf = &self.buffer[self.pos..self.pos + 8];
        self.pos += 8;
        if self.byte_order == Endianness::LittleEndian {
            LittleEndian::read_u64(buf)
        } else {
            BigEndian::read_u64(buf)
        }
    }

    pub fn read_i8(&mut self) -> i8 {
        // There's really no need for endian issues when reading single bytes.
        let val = self.buffer[self.pos] as i8;
        self.pos += 1;
        val
    }

    pub fn read_i16(&mut self) -> i16 {
        let buf = &self.buffer[self.pos..self.pos + 2];
        self.pos += 2;
        if self.byte_order == Endianness::LittleEndian {
            LittleEndian::read_i16(buf)
        } else {
            BigEndian::read_i16(buf)
        }
    }

    pub fn read_i32(&mut self) -> i32 {
        let buf = &self.buffer[self.pos..self.pos + 4];
        self.pos += 4;
        if self.byte_order == Endianness::LittleEndian {
            LittleEndian::read_i32(buf)
        } else {
            BigEndian::read_i32(buf)
        }
    }

    pub fn read_i64(&mut self) -> i64 {
        let buf = &self.buffer[self.pos..self.pos + 8];
        self.pos += 8;
        if self.byte_order == Endianness::LittleEndian {
            LittleEndian::read_i64(buf)
        } else {
            BigEndian::read_i64(buf)
        }
    }

    pub fn read_f32(&mut self) -> f32 {
        // let buf = &self.buffer[self.pos..self.pos + 4];
        self.pos += 4;
        if self.is_le {
            // LittleEndian::read_f32(buf)
            LittleEndian::read_f32(&self.buffer[self.pos - 4..self.pos])
        } else {
            // BigEndian::read_f32(buf)
            BigEndian::read_f32(&self.buffer[self.pos - 4..self.pos])
        }
    }

    pub fn as_f32_vec(&mut self) -> Vec<f32> {
        let num_values = self.buffer.len() / 4;
        let mut ret: Vec<f32> = Vec::with_capacity(num_values);
        if self.is_le {
            for a in 0..num_values {
                ret.push(LittleEndian::read_f32(&self.buffer[a*4..a*4+4]));
            }
        } else {
            for a in 0..num_values {
                ret.push(BigEndian::read_f32(&self.buffer[a*4..a*4+4]));
            }
        }
        ret
    }

    pub fn read_f64(&mut self) -> f64 {
        let buf = &self.buffer[self.pos..self.pos + 8];
        self.pos += 8;
        if self.byte_order == Endianness::LittleEndian {
            LittleEndian::read_f64(buf)
        } else {
            BigEndian::read_f64(buf)
        }
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
    pub fn from_str<'a>(val: &'a str) -> Endianness {
        let val_lc: &str = &val.to_lowercase();
        if val_lc.contains("lsb")
            || val_lc.contains("little")
            || val_lc.contains("intel")
            || val_lc.contains("least")
        {
            return Endianness::LittleEndian;
        } else {
            return Endianness::BigEndian;
        }
    }
}
