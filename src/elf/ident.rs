use super::error::{FieldErrorKind, ParserError};
use super::{Class, EV_CURRENT};
use crate::binary::{ByteReader, Encoding};

pub const IDENT_SIZE: usize = 16;
pub const ELF_MAGIC: &[u8; 4] = b"\x7FELF";

#[derive(Debug)]
pub struct Ident {
    pub class: Class,
    pub encoding: Encoding,
}

impl Ident {
    pub fn parse(bytes: &[u8]) -> Result<Ident, ParserError> {
        if bytes.len() < IDENT_SIZE {
            return Err(ParserError::InvalidSize {
                field: "ident",
                found: bytes.len(),
                expected: IDENT_SIZE,
            });
        }

        let mut reader = ByteReader::new(bytes);

        let magic = reader.read_bytes(4)?;
        if magic != ELF_MAGIC {
            return Err(ParserError::InvalidField {
                field: "magic number",
                kind: FieldErrorKind::InvalidValue,
            });
        }

        let class = match reader.read()? {
            1 => Class::Elf32,
            2 => Class::Elf64,
            _ => {
                return Err(ParserError::InvalidField {
                    field: "elf class",
                    kind: FieldErrorKind::NotInSet("1 (32-bit) or 2 (64-bit)"),
                });
            }
        };

        let encoding = match reader.read()? {
            1 => Encoding::LittleEndian,
            2 => Encoding::BigEndian,
            _ => {
                return Err(ParserError::InvalidField {
                    field: "elf data encoding",
                    kind: FieldErrorKind::NotInSet("1 (little endian) or 2 (big endian)"),
                });
            }
        };

        let version = reader.read()?;
        if version != EV_CURRENT {
            return Err(ParserError::InvalidField {
                field: "version",
                kind: FieldErrorKind::NotEqual {
                    expected: EV_CURRENT as u64,
                    found: version as u64,
                },
            });
        }

        reader.skip(9)?; // skip the rest of the ident block

        Ok(Ident { class, encoding })
    }
}
