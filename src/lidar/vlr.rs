use std::fmt;
use std::mem;

#[derive(Default, Clone, Debug)]
pub struct Vlr {
    pub reserved: u16,
    pub user_id: String,
    pub record_id: u16,
    pub record_length_after_header: u16,
    pub description: String,
    pub binary_data: Vec<u8>,
}

impl fmt::Display for Vlr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = format!("\tReserved: {}", self.reserved);
        s = s + &format!("\n\tUser ID: {}", self.user_id);
        s = s + &format!("\n\tRecord ID: {}", self.record_id);
        s = s + &format!("\n\tRecord After Length: {}", self.record_length_after_header);
        s = s + &format!("\n\tDescription: {}", self.description);
        s = s + &"\n\tVLR Data: [";
        if self.record_id == 34_735 {
            // convert the binary data to an array of u16's
            let mut i : usize = 0;
            while i < self.record_length_after_header as usize {
                let k : u16 = self.binary_data[i] as u16 | ((self.binary_data[i + 1] as u16) << 8u16);
                if i < self.record_length_after_header as usize - 2 {
                    s = s + &format!("{}, ", k);
                } else {
                    s = s + &format!("{}]", k);
                }
                i += 2;
            }
        } else if self.record_id == 34_736 {
            // convert the binary data to an array of f64's
            let mut i : usize = 0;
            while i < self.record_length_after_header as usize {
                let k: f64 = unsafe { mem::transmute::<[u8; 8], f64>([self.binary_data[i],
                    self.binary_data[i + 1], self.binary_data[i + 2], self.binary_data[i + 3],
                    self.binary_data[i + 4], self.binary_data[i + 5], self.binary_data[i + 6],
                    self.binary_data[i + 7]]) };
                i += 8;
                if i < self.record_length_after_header as usize {
                    s = s + &format!("{}, ", k);
                } else {
                    s = s + &format!("{}]", k);
                }
            }
        } else {
            // convert the data to a string
            s = s + String::from_utf8_lossy(&self.binary_data[0..self.record_length_after_header as usize]).trim() + "]";
            //s = s + "uninterpreted data]";
        }
        write!(f, "{}", s)
    }
}
