//! Stage a Jetson QSPI firmware update via UEFI capsule-on-disk when the
//! running firmware is older than the one shipped in this image.

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

/// Stage a QSPI firmware update via UEFI capsule-on-disk if the running firmware
/// is older than this image's.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Report what would be staged without touching the ESP or any EFI variable.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> ExitCode {
    init_tracing();
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(e) => {
            error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .without_time()
        .with_target(false)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
}

#[allow(clippy::unnecessary_wraps)]
fn run(_cli: &Cli) -> Result<ExitCode> {
    info!("jetson-qspi-update is not yet implemented!");
    Ok(ExitCode::FAILURE)
}
