// private sub-module defined in other files
mod byte_order_reader;
mod byte_order_writer;

// exports identifiers from private sub-modules in the current module namespace
pub use self::byte_order_reader::ByteOrderReader;
pub use self::byte_order_reader::Endianness;
// pub use self::byte_order_writer::ByteOrderWriter;

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
