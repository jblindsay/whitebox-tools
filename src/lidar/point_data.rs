use std::fmt;

#[derive(Default, Debug, Clone, Copy)]
pub struct PointBitField {
    pub value: u8,
}

impl PointBitField {
    /// Return number
    pub fn return_number(&self) -> u8 {
        let mut ret = self.value & 0b0000_0111u8;
        if ret == 0 { ret = 1; }
        ret
    }

    /// Number of returns
    pub fn number_of_returns(&self) -> u8 {
        let mut ret = (self.value & 0b0011_1000u8) >> 3u8;
        if ret == 0 { ret = 1; }
        ret
    }

    /// Scan direction flag, `true` if moving from the left side of the
    /// in-track direction to the right side and false the opposite.
    pub fn scan_direction_flag(&self) -> bool {
        //((self.value >> 6_u8) & 1_u8) == 1_u8
        (self.value & 0b0100_0000u8) == 0b0100_0000u8
    }

    /// Edge of flightline flag
    pub fn edge_of_flightline_flag(&self) -> bool {
        (self.value & 0b1000_0000u8) == 0b1000_0000u8
    }
}

impl fmt::Display for PointBitField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "return={}, num. returns={}, scan direction={}, edge of flightline={}", self.return_number(), self.number_of_returns(), self.scan_direction_flag(), self.edge_of_flightline_flag())
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ClassificationBitField {
    pub value: u8,
}

impl ClassificationBitField {
    /// Returns the classification name associated with the ClassificationBitField
    pub fn classification(&self) -> u8 {
        //self.value & 15_u8
        self.value & 0b0001_1111u8
    }

    pub fn set_classification(&mut self, value: u8) {
        self.value = (self.value & 0b1110_0000_u8) | (value & 0b0001_1111u8);
    }

    /// Returns a string represenation of the classiciation type.
    pub fn classification_string(&self) -> String {
        match self.classification() {
            0 => return String::from("Created, never classified"),
            1 => return String::from("Unclassified"),
            2 => return String::from("Ground"),
            3 => return String::from("Low vegetation"),
            4 => return String::from("Medium vegetation"),
            5 => return String::from("High vegetation"),
            6 => return String::from("Building"),
            7 => return String::from("Low point (noise)"),
            8 => return String::from("Reserved"),
            9 => return String::from("Water"),
            10 => return String::from("Rail"),
            11 => return String::from("Road surface"),
            12 => return String::from("Reserved"),
            13 => return String::from("Wire – guard (shield)"),
            14 => return String::from("Wire – conductor (phase)"),
            15 => return String::from("Transmission tower"),
            16 => return String::from("Wire-structure connector (e.g. insulator)"),
            17 => return String::from("Bridge deck"),
            18 => return String::from("High Noise"),
            19...63 => return String::from("Reserved"),
            64...255 => return String::from("User defined"),
            _ => return String::from("Unknown class"),
        }
    }

    /// Returns `true` if the point is synthetic, `false` otherwise
    pub fn synthetic(&self) -> bool {
        //((self.value >> 5_u8) & 1_u8) == 1_u8
        (self.value & 0b0010_0000u8) == 0b0010_0000u8
    }

    pub fn set_synthetic(&mut self, val: bool) {
        if val {
            self.value = self.value | 0b0010_0000u8; //(1 << 5_u8);
        } else {
            self.value = self.value & 0b1101_1111u8;
        }
    }

    /// Returns `true` if the point is a keypoint, `false` otherwise
    pub fn keypoint(&self) -> bool {
        //((self.value >> 6_u8) & 1_u8) == 1_u8
        (self.value & 0b0100_0000u8) == 0b0100_0000u8
    }

    pub fn set_keypoint(&mut self, val: bool) {
        if val {
            self.value = self.value | 0b0100_0000u8;
        } else {
            self.value = self.value & 0b1011_1111u8;;
        }
    }

    /// Returns `true` if the point is withehld, `false` otherwise
    pub fn withheld(&self) -> bool {
        //((self.value >> 7_u8) & 1_u8) == 1_u8
        (self.value & 0b1000_0000u8) == 0b1000_0000u8
    }

    pub fn set_withheld(&mut self, val: bool) {
        if val {
            self.value = self.value | 0b1000_0000u8;
        } else {
            self.value = self.value & 0b0111_1111u8;
        }
    }
}

impl fmt::Display for ClassificationBitField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "class={}, synthetic={}, keypoint={}, withheld={}", self.classification_string(), self.synthetic(), self.keypoint(), self.withheld())
    }
}

/// Returns a string represenation of a classiciation numeric value.
pub fn convert_class_val_to_class_string(value: u8) -> String {
    match value {
        0 => return String::from("Created, never classified"),
        1 => return String::from("Unclassified"),
        2 => return String::from("Ground"),
        3 => return String::from("Low vegetation"),
        4 => return String::from("Medium vegetation"),
        5 => return String::from("High vegetation"),
        6 => return String::from("Building"),
        7 => return String::from("Low point (noise)"),
        8 => return String::from("Reserved"),
        9 => return String::from("Water"),
        10 => return String::from("Rail"),
        11 => return String::from("Road surface"),
        12 => return String::from("Reserved"),
        13 => return String::from("Wire – guard (shield)"),
        14 => return String::from("Wire – conductor (phase)"),
        15 => return String::from("Transmission tower"),
        16 => return String::from("Wire-structure connector (e.g. insulator)"),
        17 => return String::from("Bridge deck"),
        18 => return String::from("High Noise"),
        19...63 => return String::from("Reserved"),
        64...255 => return String::from("User defined"),
        _ => return String::from("Unknown class"),
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PointData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub intensity: u16,
    pub bit_field: PointBitField,
    pub class_bit_field: ClassificationBitField,
    pub scan_angle: i8,
    pub user_data: u8,
    pub point_source_id: u16,
}

impl fmt::Display for PointData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(x={}, y={}, z={}, i={}\n{}\n{}\nscan_angle={}, user_data={}, source_id={})",
        self.x, self.y, self.z, self.intensity, self.bit_field, self.class_bit_field, self.scan_angle,
        self.user_data, self.point_source_id)
    }
}

impl PointData {

    /// The return number of the point.
    pub fn return_number(&self) -> u8 {
        self.bit_field.return_number()
    }

    /// Returns the number of returns associated with the point.
    pub fn number_of_returns(&self) -> u8 {
        self.bit_field.number_of_returns()
    }

    /// Returns 'true' if the the point is either a last return or the only return.
    pub fn is_late_return(&self) -> bool {
        self.return_number() == self.number_of_returns()
    }

    /// Returns 'true' if the the point is a first return (i.e. 1 of multple returns).
    pub fn is_first_return(&self) -> bool {
        if self.return_number() == 1 && self.number_of_returns() > 1 {
            return true;
        }
        false
    }

    /// Returns 'true' if the the point is an intermediate return (i.e. neither first nor last returns).
    pub fn is_intermediate_return(&self) -> bool {
        let rn = self.return_number();
        if rn > 1 && rn < self.number_of_returns() {
            return true;
        }
        false
    }

    /// Returns the classification value of the point.
    pub fn classification(&self) -> u8 {
        self.class_bit_field.classification()
    }

    /// Returns the classification string associated with the point.
    pub fn classification_string(&self) -> String {
        self.class_bit_field.classification_string()
    }

    /// Sets the classification value of the point.
    pub fn set_classification(&mut self, value: u8) {
        self.class_bit_field.set_classification(value);
    }

    /// Returns `true` if the point is synthetic, `false` otherwise
    pub fn synthetic(&self) -> bool {
        self.class_bit_field.synthetic()
    }

    pub fn set_synthetic(&mut self, val: bool) {
        self.class_bit_field.set_synthetic(val);
    }

    /// Returns `true` if the point is a keypoint, `false` otherwise
    pub fn keypoint(&self) -> bool {
        self.class_bit_field.keypoint()
    }

    pub fn set_keypoint(&mut self, val: bool) {
        self.class_bit_field.set_keypoint(val);
    }

    /// Returns `true` if the point is withehld, `false` otherwise
    pub fn withheld(&self) -> bool {
        self.class_bit_field.withheld()
    }

    pub fn set_withheld(&mut self, val: bool) {
        self.class_bit_field.set_withheld(val);
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RgbData {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct WaveformPacket {
    pub packet_descriptor_index: u8,
    pub offset_to_waveform_data: u64,
    pub waveform_packet_size: u32,
    pub ret_point_waveform_loc: f32,
    pub xt: f32,
    pub yt: f32,
    pub zt: f32,
}
