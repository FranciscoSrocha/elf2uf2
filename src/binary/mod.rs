mod error;
mod reader;
mod writer;

pub use error::BinaryError;
pub use reader::{ByteReader, Reader};
pub use writer::Writer;

#[derive(Debug, Copy, Clone)]
pub enum Encoding {
    LittleEndian,
    BigEndian,
}

impl Encoding {
    pub fn u16_from_bytes(self, bytes: [u8; 2]) -> u16 {
        match self {
            Self::LittleEndian => u16::from_le_bytes(bytes),
            Self::BigEndian => u16::from_be_bytes(bytes),
        }
    }

    pub fn u32_from_bytes(self, bytes: [u8; 4]) -> u32 {
        match self {
            Self::LittleEndian => u32::from_le_bytes(bytes),
            Self::BigEndian => u32::from_be_bytes(bytes),
        }
    }

    pub fn u64_from_bytes(self, bytes: [u8; 8]) -> u64 {
        match self {
            Self::LittleEndian => u64::from_le_bytes(bytes),
            Self::BigEndian => u64::from_be_bytes(bytes),
        }
    }
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LittleEndian => write!(f, "Little Endian"),
            Self::BigEndian => write!(f, "Big Endian"),
        }
    }
}
