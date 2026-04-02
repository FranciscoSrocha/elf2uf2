use super::error::{FieldErrorKind, InconsistentKind, ParserError};
use super::{Class, Ident};
use crate::binary::Reader;

use super::helpers::to_usize;
use super::ident::IDENT_SIZE;

pub const ELF32_HEADER_SIZE: usize = 52;
pub const ELF64_HEADER_SIZE: usize = 64;

const ELF32_PH_ENTRY_SIZE: usize = 0x20;
const ELF64_PH_ENTRY_SIZE: usize = 0x38;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq)]
pub enum FileType {
    None = 0,
    Rel = 1,
    Exec = 2,
    Dyn = 3,
    Core = 4,

    OsSpecific(u16),
    ProcSpecific(u16),
    Unknown(u16),
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Rel => write!(f, "Relocatable"),
            Self::Exec => write!(f, "Executable"),
            Self::Dyn => write!(f, "Shared Object"),
            Self::Core => write!(f, "Core"),
            Self::OsSpecific(ft) => write!(f, "OS Specific (0x{:04x})", ft),
            Self::ProcSpecific(ft) => write!(f, "Processor Specific (0x{:04x})", ft),
            Self::Unknown(ft) => write!(f, "Unknown (0x{:04x})", ft),
        }
    }
}

impl TryFrom<u16> for FileType {
    type Error = ParserError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::None,
            0x01 => Self::Rel,
            0x02 => Self::Exec,
            0x03 => Self::Dyn,
            0x04 => Self::Core,
            0xfe00..=0xfeff => Self::OsSpecific(value),
            0xff00..=0xffff => Self::ProcSpecific(value),
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug)]
pub struct Header {
    pub file_type: FileType,
    pub machine_type: u16,
    pub entry: u64,
    pub ph_offset: usize,
    pub ph_count: u16,

    header_size: usize,
    ph_entry_size: usize,
}

impl Header {
    fn parse_elf32(bytes: &[u8], ident: &Ident) -> Result<Self, ParserError> {
        if bytes.len() < ELF32_HEADER_SIZE {
            return Err(ParserError::InvalidSize {
                field: "header",
                found: bytes.len(),
                expected: ELF32_HEADER_SIZE,
            });
        }

        let mut reader = Reader::new(bytes, ident.encoding);

        reader.skip(IDENT_SIZE)?;

        let file_type = FileType::try_from(reader.read_u16()?)?;
        let machine_type = reader.read_u16()?;

        reader.skip(4)?; // skip version (checked in ident parsing)

        let entry = reader.read_u32()? as u64;
        let ph_offset_raw = reader.read_u32()? as u64;

        let ph_offset = to_usize(ph_offset_raw, "program header offset")?;

        reader.skip(4 + 4)?; // skip shoff and flags

        let header_size = reader.read_u16()? as usize;
        let ph_entry_size = reader.read_u16()? as usize;
        let ph_count = reader.read_u16()?;

        Ok(Self {
            file_type,
            machine_type,
            entry,
            ph_offset,
            ph_count,
            header_size,
            ph_entry_size,
        })
    }

    fn parse_elf64(bytes: &[u8], ident: &Ident) -> Result<Self, ParserError> {
        if bytes.len() < ELF64_HEADER_SIZE {
            return Err(ParserError::InvalidSize {
                field: "header",
                found: bytes.len(),
                expected: ELF64_HEADER_SIZE,
            });
        }

        let mut reader = Reader::new(bytes, ident.encoding);

        reader.skip(IDENT_SIZE)?;

        let file_type = FileType::try_from(reader.read_u16()?)?;
        let machine_type = reader.read_u16()?;

        reader.skip(4)?; // skip version (checked in ident parsing)

        let entry = reader.read_u64()?;
        let ph_offset_raw = reader.read_u64()?;
        let ph_offset = to_usize(ph_offset_raw, "program header offset")?;

        reader.skip(8 + 4)?; // skip shoff and flags

        let header_size = reader.read_u16()? as usize;
        let ph_entry_size = reader.read_u16()? as usize;
        let ph_count = reader.read_u16()?;

        Ok(Self {
            file_type,
            machine_type,
            entry,
            ph_offset,
            ph_count,
            header_size,
            ph_entry_size,
        })
    }

    fn validate(
        &self,
        expected_header_size: usize,
        expected_ph_entry_size: usize,
        file_size: usize,
    ) -> Result<(), ParserError> {
        if self.header_size != expected_header_size {
            return Err(ParserError::InvalidField {
                field: "header size",
                kind: FieldErrorKind::NotEqual {
                    expected: expected_header_size as u64,
                    found: self.header_size as u64,
                },
            });
        }

        if self.ph_entry_size != expected_ph_entry_size {
            return Err(ParserError::InvalidField {
                field: "program header entry size",
                kind: FieldErrorKind::NotEqual {
                    expected: expected_ph_entry_size as u64,
                    found: self.ph_entry_size as u64,
                },
            });
        }

        if self.ph_offset == 0 && self.ph_count != 0 {
            // return Err(ParserError::InvalidProgramHeaderTableOffset);
            return Err(ParserError::Inconsistent {
                fields: &["ph_offset", "ph_count"],
                kind: InconsistentKind::OffsetWithoutCount {
                    count: self.ph_count as u64,
                },
            });
        }

        let ph_count = self.ph_count as usize;
        let ph_table_size =
            ph_count
                .checked_mul(self.ph_entry_size)
                .ok_or(ParserError::Overflow {
                    op: "multiplication",
                    lhs: ph_count,
                    rhs: self.ph_entry_size,
                })?;

        let ph_table_end =
            self.ph_offset
                .checked_add(ph_table_size)
                .ok_or(ParserError::Overflow {
                    op: "addition",
                    lhs: self.ph_offset,
                    rhs: ph_table_size,
                })?;

        if ph_table_end > file_size {
            return Err(ParserError::OutOfBounds {
                what: "program header table",
                start: self.ph_offset,
                end: ph_table_end,
                size: file_size,
            });
        }

        Ok(())
    }

    pub fn parse(bytes: &[u8], ident: &Ident) -> Result<Self, ParserError> {
        let (header, expected_header_size, expected_ph_entry_size) = match ident.class {
            Class::Elf32 => (
                Self::parse_elf32(bytes, ident)?,
                ELF32_HEADER_SIZE,
                ELF32_PH_ENTRY_SIZE,
            ),
            Class::Elf64 => (
                Self::parse_elf64(bytes, ident)?,
                ELF64_HEADER_SIZE,
                ELF64_PH_ENTRY_SIZE,
            ),
        };

        header.validate(expected_header_size, expected_ph_entry_size, bytes.len())?;

        Ok(header)
    }
}
