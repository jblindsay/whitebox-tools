// private sub-module defined in other files
mod byte_order_reader;
mod byte_order_writer;

// exports identifiers from private sub-modules in the current module namespace
pub use self::byte_order_reader::ByteOrderReader;
pub use self::byte_order_reader::Endianness;
// pub use self::byte_order_writer::ByteOrderWriter;