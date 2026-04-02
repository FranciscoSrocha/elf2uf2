mod error;
mod header;
mod ident;
mod program;

use program::ProgramHeaderTable;

pub use error::ParserError;
pub use header::{FileType, Header};
pub use ident::Ident;
pub use program::{ProgramHeader, SegmentType};

const EV_CURRENT: u8 = 1;

#[derive(Debug, PartialEq, Eq)]
pub enum Class {
    Elf32,
    Elf64,
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Elf32 => write!(f, "ELF32"),
            Self::Elf64 => write!(f, "ELF64"),
        }
    }
}

pub struct Elf<'a> {
    bytes: &'a [u8],
    pub ident: Ident,
    pub header: Header,
}

impl<'a> Elf<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ParserError> {
        let ident = Ident::parse(bytes)?;
        let header = Header::parse(bytes, &ident)?;
        Ok(Self {
            bytes,
            ident,
            header,
        })
    }

    pub fn program_headers(&'a self) -> Result<ProgramHeaderTable<'a>, ParserError> {
        ProgramHeaderTable::new(self)
    }

    pub fn read_segment(&self, ph: &ProgramHeader) -> Result<&[u8], ParserError> {
        let end = ph
            .offset
            .checked_add(ph.file_size)
            .ok_or(ParserError::Overflow {
                op: "addition",
                lhs: ph.offset,
                rhs: ph.file_size,
            })?;
        Ok(&self.bytes[ph.offset..end])
    }
}

mod helpers {

    use super::ParserError;

    pub fn to_usize(value: u64, field: &'static str) -> Result<usize, ParserError> {
        usize::try_from(value).map_err(|_| ParserError::IntegerConversion {
            field,
            value,
            target: "usize",
        })
    }
}
