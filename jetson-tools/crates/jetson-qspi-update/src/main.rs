//! Stage a Jetson QSPI firmware update via UEFI capsule-on-disk when the
//! running firmware is older than the one shipped in this image.
//!
//! Idempotent per boot. Up to date or freshly staged exits 0. If UEFI's ESRT
//! reports that it already attempted a capsule for this target and failed, the
//! unit exits non-zero rather than re-staging into the same failure.

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

use jetson_qspi_update::{capsule, esp, osindications, version};

const MANUAL_FLASH_HINT: &str =
    "Flash QSPI via USB recovery with the NVIDIA L4T BSP (Linux_for_Tegra/flash.sh).";

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

fn run(cli: &Cli) -> Result<ExitCode> {
    let Some(target) = version::read_target() else {
        info!("could not read {} version, skipping", version::BL_PACKAGE);
        return Ok(ExitCode::SUCCESS);
    };
    let Some(current) = version::read_current() else {
        info!("could not read ESRT firmware version, skipping");
        return Ok(ExitCode::SUCCESS);
    };

    info!(
        "QSPI firmware current(ESRT)={current} target({})={target}",
        version::BL_PACKAGE
    );
    if current >= target {
        info!("QSPI firmware is already up to date");
        return Ok(ExitCode::SUCCESS);
    }
    info!("QSPI firmware update needed: {current} -> {target}");

    // UEFI records the outcome of the last capsule it processed in the ESRT. If
    // it already attempted this target (or newer) and the attempt failed,
    // re-staging would only repeat the failure, so surface it as a failed unit.
    if let Some(last) = version::read_last_attempt() {
        info!(
            "last capsule attempt version={} status={}",
            last.version, last.status
        );
        if last.version >= target && last.failed() {
            error!(
                "UEFI attempted a capsule for {} but it failed (ESRT last_attempt_status \
                 {}). Firmware is still {current}. {MANUAL_FLASH_HINT}",
                last.version, last.status
            );
            return Ok(ExitCode::FAILURE);
        }
    }

    let Some(board) = capsule::read_board() else {
        info!("no supported Jetson module in the device tree, skipping. {MANUAL_FLASH_HINT}");
        return Ok(ExitCode::SUCCESS);
    };
    let capsule_name = capsule::select_capsule(&board);
    info!(
        "board {:?} sku {} fab {} super={} nanoe8gb={} -> {capsule_name}",
        board.module, board.sku, board.fab, board.is_super, board.is_nanoe8gb
    );

    let Some(capsule_file) = capsule::find(capsule_name) else {
        // Update needed but the shipped capsule is missing: a packaging bug, not a skip.
        error!(
            "capsule {capsule_name} not found under {}",
            capsule::SEARCH_DIR
        );
        return Ok(ExitCode::FAILURE);
    };
    info!("selected capsule {}", capsule_file.display());

    let Some(esp_dev) = esp::find() else {
        error!("no ESP labeled {} found, cannot stage capsule", esp::LABEL);
        return Ok(ExitCode::FAILURE);
    };
    info!("ESP partition {esp_dev}");

    if cli.dry_run {
        info!(
            "[dry-run] would stage {capsule_name} to <ESP>/EFI/UpdateCapsule/ and set OsIndications"
        );
        return Ok(ExitCode::SUCCESS);
    }

    esp::stage(&esp_dev, &capsule_file, capsule_name)?;
    info!("capsule staged at <ESP>/EFI/UpdateCapsule/{capsule_name}");
    osindications::request()?;
    info!("OsIndications EFI variable set, QSPI update will be applied on next reboot");
    Ok(ExitCode::SUCCESS)
}
