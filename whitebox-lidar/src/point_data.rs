use std::fmt;

// #[derive(Default, Debug, Clone, Copy)]
// pub struct PointBitField {
//     pub value: u8,
//     pub mode_64bit: bool
// }

// impl PointBitField {
//     /// Return number
//     pub fn return_number(&self) -> u8 {
//         let flag_val = if !self.mode_64bit {
//             0b0000_0111u8
//         } else {
//             0b0000_1111u8
//         };
//         let mut ret = self.value & flag_val;
//         if ret == 0 { ret = 1; }
//         ret
//     }

//     /// Number of returns
//     pub fn number_of_returns(&self) -> u8 {
//         if !self.mode_64bit {
//             let mut ret = (self.value & 0b0011_1000u8) >> 3u8;
//             if ret == 0 { ret = 1; }
//             return ret;
//         }
//         // else 64-bit mode
//         let mut ret = (self.value & 0b1111_0000u8) >> 4u8;
//         if ret == 0 { ret = 1; }
//         ret
//     }

//     /// Scan direction flag, `true` if moving from the left side of the
//     /// in-track direction to the right side and false the opposite.
//     pub fn scan_direction_flag(&self) -> bool {
//         (self.value & 0b0100_0000u8) == 0b0100_0000u8
//     }

//     /// Edge of flightline flag
//     pub fn edge_of_flightline_flag(&self) -> bool {
//         (self.value & 0b1000_0000u8) == 0b1000_0000u8
//     }
// }

// impl fmt::Display for PointBitField {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "return={}, num. returns={}, scan direction={}, edge of flightline={}", self.return_number(), self.number_of_returns(), self.scan_direction_flag(), self.edge_of_flightline_flag())
//     }
// }

// #[derive(Default, Debug, Clone, Copy)]
// pub struct ClassificationBitField {
//     pub value: u8,
//     pub mode_64bit: bool,
//     pub extra_byte: u8, // used for 64-bit point formats
// }

// impl ClassificationBitField {
//     /// Returns the classification name associated with the ClassificationBitField
//     pub fn classification(&self) -> u8 {
//         //self.value & 15_u8
//         if !self.mode_64bit {
//             return self.value & 0b0001_1111u8
//         }
//         self.extra_byte // 64-bit mode
//     }

//     pub fn set_classification(&mut self, value: u8) {
//         if !self.mode_64bit {
//             self.value = (self.value & 0b1110_0000_u8) | (value & 0b0001_1111u8);
//         } else {
//             self.extra_byte = value;
//         }
//     }

//     /// Returns a string representation of the classiciation type.
//     pub fn classification_string(&self) -> String {
//         match self.classification() {
//             0 => return String::from("Created, never classified"),
//             1 => return String::from("Unclassified"),
//             2 => return String::from("Ground"),
//             3 => return String::from("Low vegetation"),
//             4 => return String::from("Medium vegetation"),
//             5 => return String::from("High vegetation"),
//             6 => return String::from("Building"),
//             7 => return String::from("Low point (noise)"),
//             8 => return String::from("Reserved"),
//             9 => return String::from("Water"),
//             10 => return String::from("Rail"),
//             11 => return String::from("Road surface"),
//             12 => return String::from("Reserved"),
//             13 => return String::from("Wire – guard (shield)"),
//             14 => return String::from("Wire – conductor (phase)"),
//             15 => return String::from("Transmission tower"),
//             16 => return String::from("Wire-structure connector (e.g. insulator)"),
//             17 => return String::from("Bridge deck"),
//             18 => return String::from("High noise"),
//             19...63 => return String::from("Reserved"),
//             64...255 => return String::from("User defined"),
//             _ => return String::from("Unknown class"),
//         }
//     }

//     /// Returns `true` if the point is synthetic, `false` otherwise
//     pub fn synthetic(&self) -> bool {
//         if !self.mode_64bit {
//             (self.value & 0b0010_0000u8) == 0b0010_0000u8
//     }

//     pub fn set_synthetic(&mut self, val: bool) {
//         if val {
//             self.value = self.value | 0b0010_0000u8; //(1 << 5_u8);
//         } else {
//             self.value = self.value & 0b1101_1111u8;
//         }
//     }

//     /// Returns `true` if the point is a keypoint, `false` otherwise
//     pub fn keypoint(&self) -> bool {
//         //((self.value >> 6_u8) & 1_u8) == 1_u8
//         (self.value & 0b0100_0000u8) == 0b0100_0000u8
//     }

//     pub fn set_keypoint(&mut self, val: bool) {
//         if val {
//             self.value = self.value | 0b0100_0000u8;
//         } else {
//             self.value = self.value & 0b1011_1111u8;;
//         }
//     }

//     /// Returns `true` if the point is withehld, `false` otherwise
//     pub fn withheld(&self) -> bool {
//         //((self.value >> 7_u8) & 1_u8) == 1_u8
//         (self.value & 0b1000_0000u8) == 0b1000_0000u8
//     }

//     pub fn set_withheld(&mut self, val: bool) {
//         if val {
//             self.value = self.value | 0b1000_0000u8;
//         } else {
//             self.value = self.value & 0b0111_1111u8;
//         }
//     }
// }

// impl fmt::Display for ClassificationBitField {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "class={}, synthetic={}, keypoint={}, withheld={}", self.classification_string(), self.synthetic(), self.keypoint(), self.withheld())
//     }
// }

/// Returns a string representation of a classiciation numeric value.
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
        8 => return String::from("Model Key-point (mass point)"),
        9 => return String::from("Water"),
        10 => return String::from("Rail"),
        11 => return String::from("Road surface"),
        12 => return String::from("Overlap Points"),
        13 => return String::from("Wire - guard (shield)"),
        14 => return String::from("Wire - conductor (phase)"),
        15 => return String::from("Transmission tower"),
        16 => return String::from("Wire-structure connector (e.g. insulator)"),
        17 => return String::from("Bridge deck"),
        18 => return String::from("High noise"),
        19..=63 => return String::from(&format!("Reserved ({})", value)),
        64..=255 => return String::from("User defined"),
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PointData {
    pub x: i32, // f64,
    pub y: i32, // f64,
    pub z: i32, // f64,
    pub intensity: u16,
    // pub bit_field: PointBitField,
    // pub class_bit_field: ClassificationBitField,
    pub point_bit_field: u8,
    pub class_bit_field: u8, // contains class in 32-bit point records
    pub classification: u8,  // only used in 64-bit point records
    pub scan_angle: i16,
    pub user_data: u8,
    pub point_source_id: u16,
    pub is_64bit: bool,
}

impl fmt::Display for PointData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(x={}, y={}, z={}, i={}\n{}\n{}\nscan_angle={}, user_data={}, source_id={})",
            self.x,
            self.y,
            self.z,
            self.intensity,
            self.point_bit_field,
            self.class_bit_field,
            self.scan_angle,
            self.user_data,
            self.point_source_id
        )
    }
}

impl PointData {
    /// This function provides a lossy mechanism for transferring a 64-bit LiDAR point payload into
    /// a 32-bit payload. The returns include a 32-bit formatted point_bit_field and class_bit_field.
    /// If the point data utilizes higher return numbers information
    /// will be lost in the translation to the 32-bit format.
    pub fn get_32bit_from_64bit(&self) -> (u8, u8) {
        // create a 32-bit point_bit_field including 3-bits return number, 3-bits num returns,
        // 1-bit scan direction, and 1-bit edge of flight.
        let edge_of_flight_val = if self.edge_of_flightline_flag() {
            0b1000_0000u8
        } else {
            0b0000_0000u8
        };
        let scan_direction_val = if self.scan_direction_flag() {
            0b0100_0000u8
        } else {
            0b0000_0000u8
        };
        let point_bit_field = edge_of_flight_val
            | scan_direction_val
            | ((self.number_of_returns() << 3u8) & 0b0011_1000u8)
            | (self.return_number() & 0b0000_0111u8);

        let withheld_val = if self.withheld() {
            0b1000_0000u8
        } else {
            0b0000_0000u8
        };

        let keypoint_val = if self.keypoint() {
            0b0100_0000u8
        } else {
            0b0000_0000u8
        };

        let synthetic_val = if self.synthetic() {
            0b0010_0000u8
        } else {
            0b0000_0000u8
        };

        let class_bit_field =
            withheld_val | keypoint_val | synthetic_val | (self.classification() & 0b0001_1111u8);

        (point_bit_field, class_bit_field)
    }

    /// The return number of the point.
    pub fn return_number(&self) -> u8 {
        let flag_val = if !self.is_64bit {
            0b0000_0111u8
        } else {
            0b0000_1111u8
        };
        let mut ret = self.point_bit_field & flag_val;
        if ret == 0 {
            ret = 1;
        }
        ret
    }

    /// Sets the return number associated with the point.
    pub fn set_return_number(&mut self, value: u8) {
        if !self.is_64bit {
            self.point_bit_field = (self.point_bit_field & 0b1111_1000u8) | (value & 0b0000_0111);
        } else {
            self.point_bit_field = (self.point_bit_field & 0b1111_0000u8) | (value & 0b0000_1111);
        }
    }

    /// Returns the number of returns associated with the point.
    pub fn number_of_returns(&self) -> u8 {
        if !self.is_64bit {
            let mut ret = (self.point_bit_field & 0b0011_1000u8) >> 3u8;
            if ret == 0 {
                ret = 1;
            }
            return ret;
        }
        // else 64-bit mode
        let mut ret = (self.point_bit_field & 0b1111_0000u8) >> 4u8;
        if ret == 0 {
            ret = 1;
        }
        ret
    }

    /// Sets the number of returns associated with the point.
    pub fn set_number_of_returns(&mut self, value: u8) {
        if !self.is_64bit {
            self.point_bit_field =
                (self.point_bit_field & 0b1100_0111u8) | ((value & 0b0000_0111) << 3);
        } else {
            self.point_bit_field =
                (self.point_bit_field & 0b0000_1111u8) | ((value & 0b0000_1111) << 4);
        }
    }

    /// Returns 'true' if the the point is the only return.
    pub fn is_only_return(&self) -> bool {
        self.number_of_returns() == 1
    }

    /// Returns 'true' if the the point is one of multiple returns.
    pub fn is_multiple_return(&self) -> bool {
        self.number_of_returns() > 1
    }

    /// Returns 'true' if the the point is either a first return or the only return.
    pub fn is_early_return(&self) -> bool {
        self.return_number() == 1
    }

    /// Returns 'true' if the the point is either a last return or the only return.
    pub fn is_late_return(&self) -> bool {
        self.return_number() == self.number_of_returns()
    }

    /// Returns 'true' if the the point is a last return of multiple returns.
    pub fn is_last_return(&self) -> bool {
        (self.return_number() == self.number_of_returns()) & self.is_multiple_return()
    }

    /// Returns 'true' if the the point is a first return (i.e. 1 of multiple returns).
    pub fn is_first_return(&self) -> bool {
        (self.return_number() == 1) & self.is_multiple_return()
    }

    /// Returns 'true' if the the point is an intermediate return (i.e. neither first nor last returns).
    pub fn is_intermediate_return(&self) -> bool {
        let rn = self.return_number();
        if rn > 1 && rn < self.number_of_returns() {
            return true;
        }
        false
    }

    /// Scan direction flag, `true` if moving from the left side of the
    /// in-track direction to the right side and false the opposite.
    pub fn scan_direction_flag(&self) -> bool {
        if !self.is_64bit {
            return (self.point_bit_field & 0b0100_0000u8) == 0b0100_0000u8;
        }
        // else 64-bit
        (self.class_bit_field & 0b0100_0000u8) == 0b0100_0000u8
    }

    /// Sets the scan direction flag
    pub fn set_scan_direction_flag(&mut self, value: bool) {
        if !self.is_64bit {
            if value {
                self.point_bit_field = self.point_bit_field | 0b0100_0000u8;
            } else {
                self.point_bit_field = self.point_bit_field & 0b1011_1111u8;
            }
        } else {
            if value {
                self.class_bit_field = self.class_bit_field | 0b0100_0000u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b1011_1111u8;
            }
        }
    }

    /// Returns the edge of flightline flag
    pub fn edge_of_flightline_flag(&self) -> bool {
        if !self.is_64bit {
            return (self.point_bit_field & 0b1000_0000u8) == 0b1000_0000u8;
        }
        // else 64-bit
        (self.class_bit_field & 0b1000_0000u8) == 0b1000_0000u8
    }

    /// Sets the scan direction flag
    pub fn set_edge_of_flightline_flag(&mut self, value: bool) {
        if !self.is_64bit {
            if value {
                self.point_bit_field = self.point_bit_field | 0b1000_0000u8;
            } else {
                self.point_bit_field = self.point_bit_field & !0b1000_0000u8;
            }
        } else {
            if value {
                self.class_bit_field = self.class_bit_field | 0b1000_0000u8;
            } else {
                self.class_bit_field = self.class_bit_field & !0b1000_0000u8;
            }
        }
    }

    /// Returns the classification number associated with the ClassificationBitField
    pub fn classification(&self) -> u8 {
        if !self.is_64bit {
            return self.class_bit_field & 0b0001_1111u8;
        }
        self.classification // 64-bit mode
    }

    /// Sets the point classification value.
    pub fn set_classification(&mut self, value: u8) {
        if !self.is_64bit {
            self.class_bit_field =
                (self.class_bit_field & 0b1110_0000_u8) | (value & 0b0001_1111u8);
        } else {
            self.classification = value;
        }
    }

    /// Returns a string representation of the classiciation type.
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
            13 => return String::from("Wire - guard (shield)"),
            14 => return String::from("Wire - conductor (phase)"),
            15 => return String::from("Transmission tower"),
            16 => return String::from("Wire-structure connector (e.g. insulator)"),
            17 => return String::from("Bridge deck"),
            18 => return String::from("High noise"),
            19..=63 => return String::from("Reserved"),
            64..=255 => return String::from("User defined"),
        }
    }

    /// Returns `true` if the point is synthetic, `false` otherwise
    pub fn synthetic(&self) -> bool {
        if !self.is_64bit {
            return (self.class_bit_field & 0b0010_0000u8) == 0b0010_0000u8;
        }
        (self.class_bit_field & 0b0000_0001u8) == 0b0000_0001u8
    }

    /// Sets the synthetic flag
    pub fn set_synthetic(&mut self, value: bool) {
        if !self.is_64bit {
            if value {
                self.class_bit_field = self.class_bit_field | 0b0010_0000u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b1101_1111u8;
            }
        } else {
            if value {
                self.class_bit_field = self.class_bit_field | 0b0000_0001u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b1111_1110u8;
            }
        }
    }

    /// Returns `true` if the point is a keypoint, `false` otherwise
    pub fn keypoint(&self) -> bool {
        if !self.is_64bit {
            return (self.class_bit_field & 0b0100_0000u8) == 0b0100_0000u8;
        }
        // 64-bit mode
        (self.class_bit_field & 0b0000_0010u8) == 0b0000_0010u8
    }

    /// Sets the keypoint flag
    pub fn set_keypoint(&mut self, value: bool) {
        if !self.is_64bit {
            if value {
                self.class_bit_field = self.class_bit_field | 0b0100_0000u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b1011_1111u8;
            }
        } else {
            if value {
                self.class_bit_field = self.class_bit_field | 0b0000_0010u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b1011_1101u8;
            }
        }
    }

    /// Returns `true` if the point is withheld, `false` otherwise
    pub fn withheld(&self) -> bool {
        if !self.is_64bit {
            return (self.class_bit_field & 0b1000_0000u8) == 0b1000_0000u8;
        }
        // 64-bit mode
        (self.class_bit_field & 0b0000_0100u8) == 0b0000_0100u8
    }

    /// Set withheld flag
    pub fn set_withheld(&mut self, value: bool) {
        if !self.is_64bit {
            if value {
                self.class_bit_field = self.class_bit_field | 0b1000_0000u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b0111_1111u8;
            }
        } else {
            if value {
                self.class_bit_field = self.class_bit_field | 0b0000_0100u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b1111_1011u8;
            }
        }
    }

    /// Returns `true` if the point is overlapping, `false` otherwise
    pub fn overlap(&self) -> bool {
        if self.is_64bit {
            return (self.class_bit_field & 0b0000_1000u8) == 0b0000_1000u8;
        }
        false
    }

    /// Set overlap flag
    pub fn set_overlap(&mut self, value: bool) {
        if self.is_64bit {
            if value {
                self.class_bit_field = self.class_bit_field | 0b0000_1000u8;
            } else {
                self.class_bit_field = self.class_bit_field & 0b1111_0111u8;
            }
        }
    }

    /// Returns 'true' if the the point is classified as low (7) or high (18) noise.
    pub fn is_classified_noise(&self) -> bool {
        let cls = self.classification();
        if cls == 7 || cls == 18 {
            return true;
        }
        false
    }

    /// Returns the scanner channel
    pub fn scanner_channel(&self) -> u8 {
        if self.is_64bit {
            return self.class_bit_field & 0b0011_0000u8;
        }
        0u8 // 32-bit mode only supports 1 channel systems
    }

    /// Set the scanner channel
    pub fn set_scanner_channel(&mut self, value: u8) {
        if self.is_64bit {
            self.class_bit_field =
                (self.class_bit_field & 0b1100_1111) | ((value & 0b0000_0011u8) << 4);
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColourData {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub nir: u16,
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
