use crate::binary::{Encoding, Writer};
use thiserror::Error;

const FAMILY_ID_PRESENT_FLAG: u32 = 0x00002000;
pub const BLOCK_SIZE: usize = 512;

const MAGIC_START0: u32 = 0x0a324655;
const MAGIC_START1: u32 = 0x9e5d5157;
const MAGIC_END: u32 = 0x0ab16f30;

const MAX_DATA_SIZE: usize = 476;

pub struct Uf2 {
    flags: u32,
    payload_size: u32,
    block_count: u32,
    family_id: u32,
}

impl Uf2 {
    pub fn new(
        payload_size: u32,
        block_count: u32,
        family_id: Option<u32>,
    ) -> Result<Self, Uf2Error> {
        if payload_size == 0 || payload_size > MAX_DATA_SIZE as u32 {
            return Err(Uf2Error::InvalidField {
                field: "payload size",
                kind: FieldErrorKind::OutOfRange {
                    min: 1,
                    max: MAX_DATA_SIZE as u32,
                },
            });
        }

        if !payload_size.is_multiple_of(4) {
            return Err(Uf2Error::InvalidField {
                field: "payload size",
                kind: FieldErrorKind::NotAligned { align: 4 },
            });
        }

        if block_count == 0 {
            return Err(Uf2Error::InvalidField {
                field: "block count",
                kind: FieldErrorKind::Zero,
            });
        }

        let (flags, family_id) = match family_id {
            None => (0, 0),
            Some(id) => (FAMILY_ID_PRESENT_FLAG, id),
        };

        Ok(Self {
            flags,
            payload_size,
            block_count,
            family_id,
        })
    }

    pub fn create_block(
        &self,
        addr: u32,
        block_num: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, Uf2Error> {
        if data.len() > self.payload_size as usize {
            return Err(Uf2Error::InvalidDataSize {
                found: data.len(),
                expected: self.payload_size as usize,
            });
        }

        if block_num >= self.block_count {
            return Err(Uf2Error::InvalidField {
                field: "block number",
                kind: FieldErrorKind::OutOfRange {
                    min: 0,
                    max: self.block_count - 1,
                },
            });
        }

        let mut writer = Writer::with_capacity(BLOCK_SIZE, Encoding::LittleEndian);

        writer.append_u32(MAGIC_START0);
        writer.append_u32(MAGIC_START1);
        writer.append_u32(self.flags);
        writer.append_u32(addr);
        writer.append_u32(self.payload_size);
        writer.append_u32(block_num);
        writer.append_u32(self.block_count);
        writer.append_u32(self.family_id);
        writer.append_bytes(data);
        writer.pad_to(BLOCK_SIZE - 4); // pad data until payloadsize + the rest of the uf2 block minus the magic end
        writer.append_u32(MAGIC_END);

        debug_assert_eq!(writer.len(), BLOCK_SIZE);

        Ok(writer.into_bytes())
    }
}

#[derive(Debug, Error)]
pub enum Uf2Error {
    #[error("'{field}' is invalid: {kind}")]
    InvalidField {
        field: &'static str,
        kind: FieldErrorKind,
    },

    #[error("invalid data size: got {found}, must be <= {expected}")]
    InvalidDataSize { found: usize, expected: usize },
}

#[derive(Debug)]
pub enum FieldErrorKind {
    Zero,
    NotAligned { align: u32 },
    OutOfRange { min: u32, max: u32 },
}

impl std::fmt::Display for FieldErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Zero => write!(f, "must not be zero"),
            Self::NotAligned { align } => {
                write!(f, "must be aligned to {}", align)
            }
            Self::OutOfRange { min, max } => {
                write!(f, "must be between {} and {}", min, max)
            }
        }
    }
}
