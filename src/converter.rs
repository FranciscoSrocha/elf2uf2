use std::collections::{BTreeMap, btree_map::Entry};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info, trace};

use crate::elf;
use crate::uf2;

fn align_down(addr: u32, align: u32) -> u32 {
    addr - (addr % align)
}

pub fn elf_to_uf2(
    in_path: PathBuf,
    out_path: PathBuf,
    payload_size: u32,
    family_id: Option<u32>,
) -> Result<(), ConverterError> {
    info!(
        input = %in_path.display(),
        output = %out_path.display(),
        payload_size,
        family_id = %family_id.map(|v| format!("{:#x}", v)).unwrap_or_else(|| "None".to_string()),
        "converting ELF to UF2"
    );

    // load elf
    let bytes = std::fs::read(&in_path)?;
    info!(size = %bytes.len(), "read ELF input file");
    
    let elf_file = elf::Elf::parse(&bytes)?;
    info!(
        elf_class = %elf_file.ident.class,
        file_type = %elf_file.header.file_type,
        machine_type = %format_args!("{:#X}", elf_file.header.machine_type),
        entry_point = %format_args!("{:#X}", elf_file.header.entry),
        ph_count = %elf_file.header.ph_count,
        "Elf Header",
    );

    // validate elf
    if elf_file.header.file_type != elf::FileType::Exec {
        return Err(ConverterError::InvalidElf {
            kind: InvalidElfKind::FileType(elf_file.header.file_type),
        });
    }
    if elf_file.ident.class != elf::Class::Elf32 {
        return Err(ConverterError::InvalidElf {
            kind: InvalidElfKind::Class,
        });
    }

    let ph_table: Vec<elf::ProgramHeader> =
        elf_file.program_headers()?.collect::<Result<_, _>>()?;
    info!(
        entries = %ph_table.len(),
        "program Header Table",
    );

    let mut blocks: BTreeMap<u32, Vec<u8>> = BTreeMap::new();

    for (i, ph) in ph_table.iter().enumerate() {
        info!(
            index = i,
            segment_type = %ph.segment_type,
            vaddr = %format_args!("{:#x}", ph.vaddr),
            paddr = %format_args!("{:#x}", ph.paddr),
            file_size = ph.file_size,
            mem_size = ph.mem_size,
            "processing segment"
        );
        
        if ph.segment_type != elf::SegmentType::Load || ph.file_size == 0 {
            debug!("skipping header");
            continue;
        }

        let segment_data = elf_file.read_segment(ph)?;

        let start = u32::try_from(ph.paddr).map_err(|_| ConverterError::InvalidElf {
            kind: InvalidElfKind::AddressTooLarge(ph.paddr),
        })?;
        let mut addr_offset = 0;

        let segment_size = segment_data.len();
        let payload_size_usize = payload_size as usize;

        while addr_offset < segment_size {
            let addr = start
                .checked_add(addr_offset as u32)
                .ok_or(ConverterError::AddressOverflow)?;
            let block_addr = align_down(addr, payload_size);
            let block_offset = (addr - block_addr) as usize; // safe: < payload_size

            let len = std::cmp::min(
                payload_size_usize - block_offset,
                segment_size - addr_offset,
            );

            trace!(
                addr = %format_args!("{:#x}", addr),
                block_addr = %format_args!("{:#x}", block_addr),
                block_offset,
                copy_len = len,
                "writing data into block",
            );

            let block = match blocks.entry(block_addr) {
                Entry::Occupied(entry) => {
                    debug!(
                        block_addr = %format_args!("{:#x}", block_addr),
                        offset = block_offset,
                        "Overwriting existing block data"
                    );
                    entry.into_mut()
                }
                Entry::Vacant(entry) => entry.insert(vec![0u8; payload_size_usize]),
            };

            block[block_offset..block_offset + len]
                .copy_from_slice(&segment_data[addr_offset..addr_offset + len]);

            addr_offset += len;
        }

        info!(
            block_count = blocks.len(),
            "finished segment",
        );
    }

    let block_count = u32::try_from(blocks.len()).map_err(|_| ConverterError::TooManyBlocks {
        value: blocks.len(),
    })?;
    info!(
        blocks = block_count,
        total_bytes = block_count * payload_size,
        "UF2 generation summary"
    );

    let uf2_block_generator = uf2::Uf2::new(payload_size, block_count, family_id)?;

    // create tmp file
    let tmp_name = format!(
        "uf2-tool.{}.{}.tmp",
        out_path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("Output path must have valid file name (non-empty, UTF-8)"),
        std::process::id()
    );
    let tmp_path = out_path.with_file_name(tmp_name);
    let out_file = fs::File::create(&tmp_path)?;
    
    info!(
        final_path = %out_path.display(),
        tmp_path = %tmp_path.display(),
        blocks = block_count,
        payload_size = payload_size,
        "starting UF2 file write",
    );

    let mut writer = BufWriter::new(out_file);

    for (block_num, (addr, block_data)) in blocks.iter().enumerate() {
        trace!(
            target_addr = *addr,
            block_num,
            "Writing uf2 block"
        );

        let block = uf2_block_generator.create_block(*addr, block_num as u32, block_data)?;
        writer.write_all(&block)?;
    }

    writer.flush()?;

    debug!(
        from = %tmp_path.display(),
        to = %out_path.display(),
        "renaming temporary file",
    );

    fs::rename(tmp_path, &out_path)?;
    
    info!(
        blocks = block_count,
        total_bytes = block_count * uf2::BLOCK_SIZE as u32,
        output = %out_path.display(),
        "UF2 file successfully written",
    );

    Ok(())
}

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    ElfParser(#[from] elf::ParserError),

    #[error(transparent)]
    Uf2Writer(#[from] uf2::Uf2Error),

    #[error("Invalid file: {kind}")]
    InvalidElf { kind: InvalidElfKind },

    #[error("Too many UF2 blocks: {value} exceeds u32::MAX")]
    TooManyBlocks { value: usize },

    #[error("Segment offset {value} does not fit in u32 address space")]
    OffsetTooLarge { value: usize },

    #[error("Address computation overflow")]
    AddressOverflow,
}

#[derive(Debug)]
pub enum InvalidElfKind {
    FileType(elf::FileType),
    Class,
    AddressTooLarge(u64),
}

impl std::fmt::Display for InvalidElfKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileType(ft) => write!(f, "Invalid file type {}, must be EXEC", ft),
            Self::Class => write!(f, "Elf class must be ELF32"),
            Self::AddressTooLarge(addr) => write!(
                f,
                "ELF segment address {} does not fit in UF2 address space (u32)",
                addr
            ),
        }
    }
}
