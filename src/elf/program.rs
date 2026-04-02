use super::error::{FieldErrorKind, InconsistentKind, ParserError};
use super::{Class, Elf};
use super::helpers::to_usize;
use crate::binary::Reader;

type ProgramHeaderParseFunction = fn(reader: &mut Reader) -> Result<ProgramHeader, ParserError>;

pub struct ProgramHeaderTable<'a> {
    parse_fn: ProgramHeaderParseFunction,
    ph_current_idx: u16,
    ph_count: u16,
    reader: Reader<'a>,
}

impl<'a> ProgramHeaderTable<'a> {
    pub fn new(elf_file: &'a Elf) -> Result<Self, ParserError> {
        let parse_fn = match elf_file.ident.class {
            Class::Elf32 => ProgramHeader::parse_elf32,
            Class::Elf64 => ProgramHeader::parse_elf64,
        };

        let mut reader = Reader::new(elf_file.bytes, elf_file.ident.encoding);

        // set reader to the begging of the program header table
        reader.seek(elf_file.header.ph_offset)?;

        Ok(Self {
            parse_fn,
            ph_current_idx: 0,
            ph_count: elf_file.header.ph_count,
            reader,
        })
    }
}

impl<'a> Iterator for ProgramHeaderTable<'a> {
    type Item = Result<ProgramHeader, ParserError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ph_current_idx == self.ph_count {
            return None;
        }

        match (self.parse_fn)(&mut self.reader) {
            Ok(ph) => {
                self.ph_current_idx += 1;
                Some(Ok(ph))
            }
            Err(e) => {
                self.ph_current_idx = self.ph_count; // if error stop iterator
                Some(Err(e))
            }
        }
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq)]
pub enum SegmentType {
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interp = 3,
    Note = 4,
    ShLib = 5,
    Phdr = 6,
    Tls = 7,

    OsSpecific(u32),
    ProcSpecific(u32),
    Unknown(u32),
}

impl From<u32> for SegmentType {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Null,
            1 => Self::Load,
            2 => Self::Dynamic,
            3 => Self::Interp,
            4 => Self::Note,
            5 => Self::ShLib,
            6 => Self::Phdr,
            7 => Self::Tls,
            0x60000000..=0x6FFFFFFF => Self::OsSpecific(value),
            0x70000000..=0x7FFFFFFF => Self::ProcSpecific(value),
            _ => Self::Unknown(value),
        }
    }
}

impl std::fmt::Display for SegmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (name, val) = match self {
            Self::Null => ("Null", 0),
            Self::Load => ("Loadable", 1),
            Self::Dynamic => ("Dynamic", 2),
            Self::Interp => ("Interpreter", 3),
            Self::Note => ("Note", 4),
            Self::ShLib => ("Reserved", 5),
            Self::Phdr => ("Program Header", 6),
            Self::Tls => ("TLS", 7),
            Self::OsSpecific(st) => ("OS Specific", *st),
            Self::ProcSpecific(st) => ("Processor Specific", *st),
            Self::Unknown(st) => ("Unknown", *st),
        };

        write!(f, "{} (0x{:08x})", name, val)
    }
}

pub struct ProgramHeader {
    pub segment_type: SegmentType,
    pub offset: usize,
    pub vaddr: u64,
    pub paddr: u64,
    pub file_size: usize,
    pub mem_size: u64,
    align: u64,
}

impl ProgramHeader {
    fn check_bounds(&self, file_size: usize) -> Result<(), ParserError> {
        let end = self
            .offset
            .checked_add(self.file_size)
            .ok_or(ParserError::Overflow {
                op: "addition",
                lhs: self.offset,
                rhs: self.file_size,
            })?;

        if end > file_size {
            return Err(ParserError::OutOfBounds {
                what: "Segment",
                start: self.offset,
                end,
                size: file_size,
            });
        }

        Ok(())
    }

    fn validate_load(&self, file_size: usize) -> Result<(), ParserError> {
        if self.mem_size < self.file_size as u64 {
            return Err(ParserError::Inconsistent {
                fields: &["memory size", "file size"],
                kind: InconsistentKind::SizeMismatch {
                    smaller: self.mem_size,
                    larger: self.file_size as u64,
                },
            });
        }

        self.check_bounds(file_size)?;

        if self.align <= 1 {
            return Ok(());
        }

        if !self.align.is_power_of_two() {
            return Err(ParserError::InvalidField {
                field: "align",
                kind: FieldErrorKind::NotPowerOfTwo { value: self.align },
            });
        }

        let offset = self.offset as u64;
        if offset % self.align != self.vaddr % self.align {
            return Err(ParserError::Inconsistent {
                fields: &["offset", "virtual address"],
                kind: InconsistentKind::Misaligned {
                    align: self.align,
                    a: self.offset as u64,
                    b: self.vaddr,
                },
            });
        }

        Ok(())
    }

    fn read_elf32(reader: &mut Reader) -> Result<Self, ParserError> {
        let segment_type = SegmentType::from(reader.read_u32()?);

        let offset_raw = reader.read_u32()? as u64;
        let offset = to_usize(offset_raw, "segment offset")?;

        let vaddr = reader.read_u32()? as u64;
        let paddr = reader.read_u32()? as u64; // physical address (skipped)

        let file_size_raw = reader.read_u32()? as u64;
        let file_size = to_usize(file_size_raw, "file size")?;

        let mem_size = reader.read_u32()? as u64;

        reader.skip(4)?; // flags (skipped)

        let align = reader.read_u32()? as u64;

        Ok(Self {
            segment_type,
            offset,
            vaddr,
            paddr,
            file_size,
            mem_size,
            align,
        })
    }

    fn read_elf64(reader: &mut Reader) -> Result<Self, ParserError> {
        let segment_type = SegmentType::from(reader.read_u32()?);

        reader.skip(4)?; // skip flags

        let offset_raw = reader.read_u64()?;
        let offset = to_usize(offset_raw, "segment offset")?;

        let vaddr = reader.read_u64()?;
        let paddr = reader.read_u64()?; // skip physical address

        let file_size_raw = reader.read_u64()?;
        let file_size = to_usize(file_size_raw, "file size")?;

        let mem_size = reader.read_u64()?;
        let align = reader.read_u64()?;

        Ok(Self {
            segment_type,
            offset,
            vaddr,
            paddr,
            file_size,
            mem_size,
            align,
        })
    }

    fn validate(self, file_size: usize) -> Result<Self, ParserError> {
        match self.segment_type {
            SegmentType::Null => {}
            SegmentType::Load => self.validate_load(file_size)?,
            _ => self.check_bounds(file_size)?,
        }

        Ok(self)
    }

    fn parse_elf32(reader: &mut Reader) -> Result<Self, ParserError> {
        let ph = ProgramHeader::read_elf32(reader)?.validate(reader.len())?;
        Ok(ph)
    }

    fn parse_elf64(reader: &mut Reader) -> Result<Self, ParserError> {
        let ph = ProgramHeader::read_elf64(reader)?.validate(reader.len())?;
        Ok(ph)
    }
}
