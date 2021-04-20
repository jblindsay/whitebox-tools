#[derive(Clone, PartialEq)]
pub enum ZlidarCompression {
    None,
    Deflate { level: u8 },
    Brotli { level: u8 },
}

impl Default for ZlidarCompression {
    // fn default() -> Self { ZlidarCompression::None }
    fn default() -> Self { ZlidarCompression::Brotli{ level: 5 } }
}