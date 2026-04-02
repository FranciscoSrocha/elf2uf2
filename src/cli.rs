use clap::Parser;
use clap_num::maybe_hex;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "elf2uf2",
    version = "0.1.0",
    about = "Converts ELF files into UF2 format for microcontrollers"
)]
pub struct Cli {
    /// input file (.elf)
    #[arg(value_parser = input_file_valid)]
    pub input: PathBuf,

    /// output file (.uf2)
    #[arg(short, long, default_value = "a.uf2", value_parser = output_file_valid)]
    pub output: PathBuf,

    /// payload size in bytes per UF2 block (must be <= 476 and multiple of 4)
    #[arg(long, value_parser = payload_size_in_range, default_value = "256")]
    pub payload_size: u32,

    /// MCU family ID
    #[arg(long, value_parser = maybe_hex::<u32>)]
    pub family_id: Option<u32>,

    /// -v info mode, -vv debug
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

fn payload_size_in_range(s: &str) -> Result<u32, String> {
    let value = maybe_hex::<u32>(s)?;

    if value == 0 || value > 476 {
        return Err("payload size must be between 1 and 476".into());
    }

    if value % 4 != 0 {
        return Err("payload size must be a multiple of 4".into());
    }

    Ok(value)
}

fn input_file_valid(p: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(p);

    if !path.exists() {
        return Err("input file does not exist".into());
    }

    if !path.is_file() {
        return Err("input path is not a file".into());
    }

    if path.extension().and_then(|s| s.to_str()) != Some("elf") {
        return Err("input file must have .elf extension".into());
    }

    Ok(path)
}

fn output_file_valid(p: &str) -> Result<PathBuf, String> {
    let mut path = PathBuf::from(p);

    if path.parent().is_none_or(|p| p.as_os_str().is_empty()) {
        path = Path::new(".").join(path);
    }

    let parent = path
        .parent()
        .expect("Normalized path should always have a parent");

    if !parent.exists() {
        return Err("output directory does not exist".into());
    }
    if !parent.is_dir() {
        return Err("output parent is not a directory".into());
    }

    // validate file extension
    if path.extension().and_then(|s| s.to_str()) != Some("uf2") {
        return Err("output file must have .uf2 extension".into());
    }

    Ok(path)
}
