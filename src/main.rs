use clap::Parser;
use std::fs::{read_dir, remove_file};
use std::path::Path;
use tracing::{Level, debug, error, info, warn};

use uf2_tool::cli::Cli;
use uf2_tool::converter;

fn init_logging(verbose: u8) {
    let lvl = match verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(lvl)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .without_time()
        .with_level(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER)
        .init();
}

fn delete_tmp_files(dir: &Path) -> Result<(), std::io::Error> {
    info!(
        dir = %dir.display(),
        "deleting tmp file",
    );

    let mut tmp_file_count = 0;

    for entry in read_dir(dir)? {
        let path = entry?.path();

        if path.is_file()
            && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            && file_name.starts_with("uf2-tool.")
            && file_name.ends_with(".tmp")
        {
            match remove_file(&path) {
                Ok(_) => {
                    tmp_file_count += 1;
                    debug!(file = %path.display(), "Deleted tmp file");
                }
                Err(e) => {
                    warn!(file = %path.display(), error = %e, "Failed to delete tmp file");
                }
            };
        }
    }
    info!(deleted = tmp_file_count, "Temporary file cleanup complete");
    Ok(())
}

fn run() -> Result<(), converter::ConverterError> {
    let args = Cli::parse();
    init_logging(args.verbose);

    // delete tmp files
    if let Err(e) = delete_tmp_files(
        args.output
            .parent()
            .expect("Parent should exist and be different of \"\""),
    ) {
        warn!(error = %e, "failed to clean temporary files");
    };

    converter::elf_to_uf2(args.input, args.output, args.payload_size, args.family_id)?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}
