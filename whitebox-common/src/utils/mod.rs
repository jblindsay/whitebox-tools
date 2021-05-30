// private sub-module defined in other files
mod byte_order_reader;
mod byte_order_writer;

// exports identifiers from private sub-modules in the current module namespace
pub use self::byte_order_reader::ByteOrderReader;
pub use self::byte_order_reader::Endianness;
pub use self::byte_order_writer::ByteOrderWriter;

use std::time::Instant;

/// Returns a formatted string of elapsed time, e.g.
/// `1min 34s 852ms`
pub fn get_formatted_elapsed_time(instant: Instant) -> String {
    let dur = instant.elapsed();
    let minutes = dur.as_secs() / 60;
    let sub_sec = dur.as_secs() % 60;
    let sub_milli = dur.subsec_millis();
    if minutes > 0 {
        return format!("{}min {}.{}s", minutes, sub_sec, sub_milli);
    }
    format!("{}.{}s", sub_sec, sub_milli)
}

pub fn wrapped_print(val: &str, width: usize) {
    let split_val1: Vec<&str> = val.split("\n\n").collect(); // paragraphs
    for i in 0..split_val1.len() {
        let s = split_val1[i].replace("\n", "");
        let split_val2: Vec<&str> = s.split(" ").collect();
        let mut s1 = String::new();
        for s2 in split_val2 {
            if s1.len() + s2.len() + 1 <= width {
                s1 = format!("{} {}", s1, s2).trim().to_string();
            } else {
                println!("{}", s1);
                s1 = s2.to_string();
            }
        }
        if i < split_val1.len()-1 {
            println!("{}\n", s1);
        } else {
            println!("{}", s1);
        }
    }
}

pub fn wrapped_text(val: &str, width: usize) -> String {
    let split_val1: Vec<&str> = val.split("\n\n").collect(); // paragraphs
    let mut ret = String::new();
    for i in 0..split_val1.len() {
        let s = split_val1[i].replace("\n", "");
        let split_val2: Vec<&str> = s.split(" ").collect();
        let mut s1 = String::new();
        for s2 in split_val2 {
            if s1.len() + s2.len() + 1 <= width {
                s1 = format!("{} {}", s1, s2).trim().to_string();
            } else {
                ret.push_str(&format!("{}\n", s1));
                s1 = s2.to_string();
            }
        }
        if i < split_val1.len()-1 {
            ret.push_str(&format!("{}\n", s1));
        } else {
            ret.push_str(&s1);
        }
    }
    ret
}

// Derived from: https://stackoverflow.com/questions/176137/java-convert-lat-lon-to-utm
// Testing shows that this produces UTM coordinates that are within a few centimeters of 
// other lat/long-UTM conversion libraries.
pub fn deg_to_utm(latitude: f64, longitude: f64) -> (f64, f64, isize, char) {
    let zone = (longitude / 6.0 + 31.0).floor();
    let letter = if latitude < -72.0 {
        'C'
    } else if latitude < -64.0 {
        'D'
    } else if latitude < -56.0 {
        'E'
    } else if latitude < -48.0 {
        'F'
    } else if latitude < -40.0 {
        'G'
    } else if latitude < -32.0 {
        'H'
    } else if latitude < -24.0 {
        'J'
    } else if latitude < -16.0 {
        'K'
    } else if latitude < -8.0 {
        'L'
    } else if latitude < 0.0 {
        'M'
    } else if latitude < 8.0 {
        'N'
    } else if latitude < 16.0 {
        'P'
    } else if latitude < 24.0 {
        'Q'
    } else if latitude < 32.0 {
        'R'
    } else if latitude < 40.0 {
        'S'
    } else if latitude < 48.0 {
        'T'
    } else if latitude < 56.0 {
        'U'
    } else if latitude < 64.0 {
        'V'
    } else if latitude < 72.0 {
        'W'
    } else {
        'X'
    };

    let lat = latitude.to_radians();
    let lon = longitude.to_radians();
    let val1 = (6.0 * zone - 183.0).to_radians();
    let val2 = (lon - val1).sin();
    let val3 = lat.cos();
    let val4 = (2.0 * lat).sin();
    let easting = 0.5 * ((1.0 + val3 * val2) / (1.0 - val3 * val2)).ln() * 0.9996 * 6399593.62 / (1.0 + 0.0820944379 * 0.0820944379 * val3 * val3).sqrt() * (1.0 + 0.0820944379 * 0.0820944379 / 2.0 * (0.5 * ((1.0 + val3 * val2) / (1.0 - val3 * val2)).ln()).powi(2) * val3 * val3 / 3.0) + 500000.0;
    let mut northing = ((lat.tan() / (lon-val1).cos()).atan() - lat) * 0.9996 * 6399593.625 / (1.0 + 0.006739496742 * val3 * val3).sqrt() * (1.0 + 0.006739496742 / 2.0 * (0.5 * ((1.0 + val3 * (lon - val1).sin()) / (1.0 - val3 * (lon - val1).sin())).ln()).powi(2) * val3 * val3) + 0.9996 * 6399593.625 * (lat - 0.005054622556 * (lat + val4 / 2.0) + 4.258201531e-05 * (3.0 * (lat + val4 / 2.0) + val4 * val3 * val3) / 4.0 - 1.674057895e-07 * (5.0 * (3.0 * (lat + val4 / 2.0) + val4 * val3 * val3)/4.0 + val4 * val3 * val3 * val3 * val3) / 3.0);
    
    if letter < 'M' {
        northing += 10000000.0;
    }
    
    (easting, northing, zone as isize, letter)
}

// e.g "35 R 312915.84 4451481.33"
#[allow(nonstandard_style, unused_parens)]
pub fn utm_to_deg(zone: isize, letter: char, easting: f64, northing: f64) -> (f64, f64) {
    // let parts = utm.split(" ");
    // let zone = parts[0].parse::<usize>().expect("Error parsing UTM string");
    // char Letter=parts[1].toUpperCase(Locale.ENGLISH).charAt(0);
    // let easting= Double.parseDouble(parts[2]);
    // let northing=Double.parseDouble(parts[3]);

    let hem = if letter > 'M' { 'N' } else { 'S' };

    let north = if hem == 'S' {
        northing - 10000000.0
    } else {
        northing
    };
    // let val1 = 0.9996 * 6399593.625;
    // let val2 = north / 6366197.724 / 0.9996;
    // let val3 = 1.0 + 0.006739496742 * val2.cos().powi(2);

    let latitude = (north / 6366197.724 / 0.9996
        + (1.0 + 0.006739496742 * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
            - 0.006739496742
                * f64::sin(north / 6366197.724 / 0.9996)
                * f64::cos(north / 6366197.724 / 0.9996)
                * (f64::atan(
                    f64::cos(f64::atan(
                        (f64::exp(
                            (easting - 500000.0)
                                / (0.9996 * 6399593.625
                                    / f64::sqrt(
                                        (1.0 + 0.006739496742
                                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                    ))
                                * (1.0
                                    - 0.006739496742
                                        * f64::powi(
                                            (easting - 500000.0)
                                                / (0.9996 * 6399593.625
                                                    / f64::sqrt(
                                                        (1.0 + 0.006739496742
                                                            * f64::powi(
                                                                f64::cos(
                                                                    north / 6366197.724 / 0.9996,
                                                                ),
                                                                2,
                                                            )),
                                                    )),
                                            2,
                                        )
                                        / 2.0
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
                                        / 3.0),
                        ) - f64::exp(
                            -(easting - 500000.0)
                                / (0.9996 * 6399593.625
                                    / f64::sqrt(
                                        (1.0 + 0.006739496742
                                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                    ))
                                * (1.0 - 0.006739496742
                                    * f64::powi(
                                        (easting - 500000.0)
                                            / (0.9996 * 6399593.625
                                                / f64::sqrt(
                                                    (1.0 + 0.006739496742
                                                        * f64::powi(
                                                            f64::cos(north / 6366197.724 / 0.9996),
                                                            2,
                                                        )),
                                                )),
                                        2,
                                    )
                                    / 2.0
                                    * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
                                    / 3.0),
                        )) / 2.0
                            / f64::cos(
                                (north
                                    - 0.9996
                                        * 6399593.625
                                        * (north / 6366197.724 / 0.9996
                                            - 0.006739496742 * 3.0 / 4.0
                                                * (north / 6366197.724 / 0.9996
                                                    + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                        / 2.0)
                                            + f64::powi(0.006739496742 * 3.0 / 4.0, 2) * 5.0 / 3.0
                                                * (3.0
                                                    * (north / 6366197.724 / 0.9996
                                                        + f64::sin(
                                                            2.0 * north / 6366197.724 / 0.9996,
                                                        ) / 2.0)
                                                    + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                        * f64::powi(
                                                            f64::cos(north / 6366197.724 / 0.9996),
                                                            2,
                                                        ))
                                                / 4.0
                                            - f64::powi(0.006739496742 * 3.0 / 4.0, 3) * 35.0 / 27.0
                                                * (5.0 * (3.0
                                                    * (north / 6366197.724 / 0.9996
                                                        + f64::sin(
                                                            2.0 * north / 6366197.724 / 0.9996,
                                                        ) / 2.0)
                                                    + f64::sin(
                                                        2.0 * north / 6366197.724 / 0.9996,
                                                    ) * f64::powi(
                                                        f64::cos(north / 6366197.724 / 0.9996),
                                                        2,
                                                    ))
                                                    / 4.0
                                                    + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                        * f64::powi(
                                                            f64::cos(north / 6366197.724 / 0.9996),
                                                            2,
                                                        )
                                                        * f64::powi(
                                                            f64::cos(north / 6366197.724 / 0.9996),
                                                            2,
                                                        ))
                                                / 3.0))
                                    / (0.9996 * 6399593.625
                                        / f64::sqrt(
                                            (1.0 + 0.006739496742
                                                * f64::powi(
                                                    f64::cos(north / 6366197.724 / 0.9996),
                                                    2,
                                                )),
                                        ))
                                    * (1.0
                                        - 0.006739496742
                                            * f64::powi(
                                                (easting - 500000.0)
                                                    / (0.9996 * 6399593.625
                                                        / f64::sqrt(
                                                            (1.0 + 0.006739496742
                                                                * f64::powi(
                                                                    f64::cos(
                                                                        north
                                                                            / 6366197.724
                                                                            / 0.9996,
                                                                    ),
                                                                    2,
                                                                )),
                                                        )),
                                                2,
                                            )
                                            / 2.0
                                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                                    + north / 6366197.724 / 0.9996,
                            ),
                    )) * f64::tan(
                        (north
                            - 0.9996
                                * 6399593.625
                                * (north / 6366197.724 / 0.9996
                                    - 0.006739496742 * 3.0 / 4.0
                                        * (north / 6366197.724 / 0.9996
                                            + f64::sin(2.0 * north / 6366197.724 / 0.9996) / 2.0)
                                    + f64::powi(0.006739496742 * 3.0 / 4.0, 2) * 5.0 / 3.0
                                        * (3.0
                                            * (north / 6366197.724 / 0.9996
                                                + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                    / 2.0)
                                            + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                * f64::powi(
                                                    f64::cos(north / 6366197.724 / 0.9996),
                                                    2,
                                                ))
                                        / 4.0
                                    - f64::powi(0.006739496742 * 3.0 / 4.0, 3) * 35.0 / 27.0
                                        * (5.0 * (3.0
                                            * (north / 6366197.724 / 0.9996
                                                + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                    / 2.0)
                                            + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                * f64::powi(
                                                    f64::cos(north / 6366197.724 / 0.9996),
                                                    2,
                                                ))
                                            / 4.0
                                            + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                * f64::powi(
                                                    f64::cos(north / 6366197.724 / 0.9996),
                                                    2,
                                                )
                                                * f64::powi(
                                                    f64::cos(north / 6366197.724 / 0.9996),
                                                    2,
                                                ))
                                        / 3.0))
                            / (0.9996 * 6399593.625
                                / f64::sqrt(
                                    (1.0 + 0.006739496742
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                ))
                            * (1.0
                                - 0.006739496742
                                    * f64::powi(
                                        (easting - 500000.0)
                                            / (0.9996 * 6399593.625
                                                / f64::sqrt(
                                                    (1.0 + 0.006739496742
                                                        * f64::powi(
                                                            f64::cos(north / 6366197.724 / 0.9996),
                                                            2,
                                                        )),
                                                )),
                                        2,
                                    )
                                    / 2.0
                                    * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                            + north / 6366197.724 / 0.9996,
                    ),
                ) - north / 6366197.724 / 0.9996)
                * 3.0
                / 2.0)
            * (f64::atan(
                f64::cos(f64::atan(
                    (f64::exp(
                        (easting - 500000.0)
                            / (0.9996 * 6399593.625
                                / f64::sqrt(
                                    (1.0 + 0.006739496742
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                ))
                            * (1.0
                                - 0.006739496742
                                    * f64::powi(
                                        (easting - 500000.0)
                                            / (0.9996 * 6399593.625
                                                / f64::sqrt(
                                                    (1.0 + 0.006739496742
                                                        * f64::powi(
                                                            f64::cos(north / 6366197.724 / 0.9996),
                                                            2,
                                                        )),
                                                )),
                                        2,
                                    )
                                    / 2.0
                                    * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
                                    / 3.0),
                    ) - f64::exp(
                        -(easting - 500000.0)
                            / (0.9996 * 6399593.625
                                / f64::sqrt(
                                    (1.0 + 0.006739496742
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                ))
                            * (1.0
                                - 0.006739496742
                                    * f64::powi(
                                        (easting - 500000.0)
                                            / (0.9996 * 6399593.625
                                                / f64::sqrt(
                                                    (1.0 + 0.006739496742
                                                        * f64::powi(
                                                            f64::cos(north / 6366197.724 / 0.9996),
                                                            2,
                                                        )),
                                                )),
                                        2,
                                    )
                                    / 2.0
                                    * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
                                    / 3.0),
                    )) / 2.0
                        / f64::cos(
                            (north
                                - 0.9996
                                    * 6399593.625
                                    * (north / 6366197.724 / 0.9996
                                        - 0.006739496742 * 3.0 / 4.0
                                            * (north / 6366197.724 / 0.9996
                                                + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                    / 2.0)
                                        + f64::powi(0.006739496742 * 3.0 / 4.0, 2) * 5.0 / 3.0
                                            * (3.0
                                                * (north / 6366197.724 / 0.9996
                                                    + f64::sin(
                                                        2.0 * north / 6366197.724 / 0.9996,
                                                    ) / 2.0)
                                                + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                    * f64::powi(
                                                        f64::cos(north / 6366197.724 / 0.9996),
                                                        2,
                                                    ))
                                            / 4.0
                                        - f64::powi(0.006739496742 * 3.0 / 4.0, 3) * 35.0 / 27.0
                                            * (5.0 * (3.0
                                                * (north / 6366197.724 / 0.9996
                                                    + f64::sin(
                                                        2.0 * north / 6366197.724 / 0.9996,
                                                    ) / 2.0)
                                                + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                    * f64::powi(
                                                        f64::cos(north / 6366197.724 / 0.9996),
                                                        2,
                                                    ))
                                                / 4.0
                                                + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                                    * f64::powi(
                                                        f64::cos(north / 6366197.724 / 0.9996),
                                                        2,
                                                    )
                                                    * f64::powi(
                                                        f64::cos(north / 6366197.724 / 0.9996),
                                                        2,
                                                    ))
                                            / 3.0))
                                / (0.9996 * 6399593.625
                                    / f64::sqrt(
                                        (1.0 + 0.006739496742
                                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                    ))
                                * (1.0
                                    - 0.006739496742
                                        * f64::powi(
                                            (easting - 500000.0)
                                                / (0.9996 * 6399593.625
                                                    / f64::sqrt(
                                                        (1.0 + 0.006739496742
                                                            * f64::powi(
                                                                f64::cos(
                                                                    north / 6366197.724 / 0.9996,
                                                                ),
                                                                2,
                                                            )),
                                                    )),
                                            2,
                                        )
                                        / 2.0
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                                + north / 6366197.724 / 0.9996,
                        ),
                )) * f64::tan(
                    (north
                        - 0.9996
                            * 6399593.625
                            * (north / 6366197.724 / 0.9996
                                - 0.006739496742 * 3.0 / 4.0
                                    * (north / 6366197.724 / 0.9996
                                        + f64::sin(2.0 * north / 6366197.724 / 0.9996) / 2.0)
                                + f64::powi(0.006739496742 * 3.0 / 4.0, 2) * 5.0 / 3.0
                                    * (3.0
                                        * (north / 6366197.724 / 0.9996
                                            + f64::sin(2.0 * north / 6366197.724 / 0.9996) / 2.0)
                                        + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                            * f64::powi(
                                                f64::cos(north / 6366197.724 / 0.9996),
                                                2,
                                            ))
                                    / 4.0
                                - f64::powi(0.006739496742 * 3.0 / 4.0, 3) * 35.0 / 27.0
                                    * (5.0 * (3.0
                                        * (north / 6366197.724 / 0.9996
                                            + f64::sin(2.0 * north / 6366197.724 / 0.9996) / 2.0)
                                        + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                            * f64::powi(
                                                f64::cos(north / 6366197.724 / 0.9996),
                                                2,
                                            ))
                                        / 4.0
                                        + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                            * f64::powi(
                                                f64::cos(north / 6366197.724 / 0.9996),
                                                2,
                                            )
                                            * f64::powi(
                                                f64::cos(north / 6366197.724 / 0.9996),
                                                2,
                                            ))
                                    / 3.0))
                        / (0.9996 * 6399593.625
                            / f64::sqrt(
                                (1.0 + 0.006739496742
                                    * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                            ))
                        * (1.0
                            - 0.006739496742
                                * f64::powi(
                                    (easting - 500000.0)
                                        / (0.9996 * 6399593.625
                                            / f64::sqrt(
                                                (1.0 + 0.006739496742
                                                    * f64::powi(
                                                        f64::cos(north / 6366197.724 / 0.9996),
                                                        2,
                                                    )),
                                            )),
                                    2,
                                )
                                / 2.0
                                * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                        + north / 6366197.724 / 0.9996,
                ),
            ) - north / 6366197.724 / 0.9996))
        * 180.0
        / std::f64::consts::PI;

    let longitude = f64::atan(
        (f64::exp(
            (easting - 500000.0)
                / (0.9996 * 6399593.625
                    / f64::sqrt(
                        (1.0 + 0.006739496742
                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                    ))
                * (1.0
                    - 0.006739496742
                        * f64::powi(
                            (easting - 500000.0)
                                / (0.9996 * 6399593.625
                                    / f64::sqrt(
                                        (1.0 + 0.006739496742
                                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                    )),
                            2,
                        )
                        / 2.0
                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
                        / 3.0),
        ) - f64::exp(
            -(easting - 500000.0)
                / (0.9996 * 6399593.625
                    / f64::sqrt(
                        (1.0 + 0.006739496742
                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                    ))
                * (1.0
                    - 0.006739496742
                        * f64::powi(
                            (easting - 500000.0)
                                / (0.9996 * 6399593.625
                                    / f64::sqrt(
                                        (1.0 + 0.006739496742
                                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                                    )),
                            2,
                        )
                        / 2.0
                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
                        / 3.0),
        )) / 2.0
            / f64::cos(
                (north
                    - 0.9996
                        * 6399593.625
                        * (north / 6366197.724 / 0.9996
                            - 0.006739496742 * 3.0 / 4.0
                                * (north / 6366197.724 / 0.9996
                                    + f64::sin(2.0 * north / 6366197.724 / 0.9996) / 2.0)
                            + f64::powi(0.006739496742 * 3.0 / 4.0, 2) * 5.0 / 3.0
                                * (3.0
                                    * (north / 6366197.724 / 0.9996
                                        + f64::sin(2.0 * north / 6366197.724 / 0.9996) / 2.0)
                                    + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                                / 4.0
                            - f64::powi(0.006739496742 * 3.0 / 4.0, 3) * 35.0 / 27.0
                                * (5.0 * (3.0
                                    * (north / 6366197.724 / 0.9996
                                        + f64::sin(2.0 * north / 6366197.724 / 0.9996) / 2.0)
                                    + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                                    / 4.0
                                    + f64::sin(2.0 * north / 6366197.724 / 0.9996)
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)
                                        * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                                / 3.0))
                    / (0.9996 * 6399593.625
                        / f64::sqrt(
                            (1.0 + 0.006739496742
                                * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2)),
                        ))
                    * (1.0
                        - 0.006739496742
                            * f64::powi(
                                (easting - 500000.0)
                                    / (0.9996 * 6399593.625
                                        / f64::sqrt(
                                            (1.0 + 0.006739496742
                                                * f64::powi(
                                                    f64::cos(north / 6366197.724 / 0.9996),
                                                    2,
                                                )),
                                        )),
                                2,
                            )
                            / 2.0
                            * f64::powi(f64::cos(north / 6366197.724 / 0.9996), 2))
                    + north / 6366197.724 / 0.9996,
            ),
    ) * 180.0
        / std::f64::consts::PI
        + zone as f64 * 6.0
        - 183.0;

    (latitude, longitude)
}
