//! Stage a Jetson QSPI firmware update via UEFI capsule-on-disk when the
//! running firmware is older than the one shipped in this image.
//!
//! On bootc/composefs the ESP is not mounted at runtime, so we find it by label
//! and mount it transiently, like bootc's own `mount_esp()`.
//!
//! Idempotent per boot. Up to date or freshly staged exits 0. A capsule staged
//! on a prior boot that UEFI never applied exits non-zero, so a stuck update
//! surfaces as a failed unit instead of re-staging forever.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result};
use rustix::fs::{IFlags, ioctl_getflags, ioctl_setflags};
use rustix::mount::{MountFlags, UnmountFlags, mount, unmount};

use jetson_qspi_update::nvbootctrl::{self, CapsuleStatus};
use jetson_qspi_update::{capsule, osindications, version};

/// Deb package that ships the QSPI bootloader firmware.
const BL_PACKAGE: &str = "nvidia-l4t-bootloader";
const ESRT_FW_VERSION: &str = "/sys/firmware/efi/esrt/entries/entry0/fw_version";
const CAPSULE_SEARCH_DIR: &str = "/opt/ota_package";
const DT_IDS: &str = "/proc/device-tree/chosen/ids";
const EFIVARS_DIR: &str = "/sys/firmware/efi/efivars";
/// NVIDIA's compat-spec board name, under `gNVIDIAPublicVariableGuid`.
const COMPAT_SPEC_VAR: &str = "TegraPlatformCompatSpec-781e084c-a330-417c-b678-38e696380cb9";
/// Label bootc assigns the ESP at install.
const ESP_LABEL: &str = "EFI-SYSTEM";

const MANUAL_FLASH_HINT: &str =
    "Flash QSPI via USB recovery with the NVIDIA L4T BSP (Linux_for_Tegra/flash.sh).";

struct Opts {
    dry_run: bool,
}

fn main() -> ExitCode {
    let opts = match parse_opts() {
        Ok(opts) => opts,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    match run(&opts) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn parse_opts() -> Result<Opts> {
    let mut dry_run = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "-h" | "--help" => {
                println!(
                    "Usage: jetson-qspi-update [--dry-run]\n\n\
                     Stage a QSPI firmware update via UEFI capsule-on-disk if the\n\
                     running firmware is older than this image's.\n\n\
                     --dry-run  Report what would be staged without touching the ESP\n\
                     \x20          or any EFI variable."
                );
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }
    Ok(Opts { dry_run })
}

fn run(opts: &Opts) -> Result<ExitCode> {
    let Some(target) = read_target_version() else {
        println!("Could not read {BL_PACKAGE} version. Skipping.");
        return Ok(ExitCode::SUCCESS);
    };
    let Some(current) = read_current_version() else {
        println!("Could not read ESRT firmware version. Skipping.");
        return Ok(ExitCode::SUCCESS);
    };

    println!("QSPI firmware: current(ESRT)={current} target({BL_PACKAGE})={target}");
    if current >= target {
        println!("QSPI firmware is already up to date.");
        return Ok(ExitCode::SUCCESS);
    }
    println!("QSPI firmware update needed: {current} -> {target}");

    // A capsule attempted on a prior boot while firmware is still old means a
    // stuck update: surface it below rather than re-staging silently.
    let status = read_capsule_status();
    println!(
        "Capsule update status (nvbootctrl): {}",
        status.map_or_else(|| "unknown".to_string(), |s| format!("{s:?}"))
    );

    let os_indications = Path::new(EFIVARS_DIR).join(osindications::var_name());
    if os_indications_pending(&os_indications) {
        eprintln!("A capsule was staged on a previous boot but UEFI has not consumed it");
        eprintln!("(OsIndications still set, firmware still {current}). Capsule-on-disk is");
        eprintln!("not applying on this device. {MANUAL_FLASH_HINT}");
        return Ok(ExitCode::FAILURE);
    }
    if status.is_some_and(CapsuleStatus::is_failure) {
        eprintln!("UEFI attempted the capsule but firmware is still {current} (nvbootctrl");
        eprintln!("capsule status {status:?}). The QSPI update failed. {MANUAL_FLASH_HINT}");
        return Ok(ExitCode::FAILURE);
    }

    let Some(board) = read_board() else {
        println!("No supported Jetson module in the device tree. Skipping.");
        println!("{MANUAL_FLASH_HINT}");
        return Ok(ExitCode::SUCCESS);
    };
    let capsule_name = capsule::select_capsule(&board);
    println!(
        "Board {:?} sku {} fab {} super={} nanoe8gb={} -> {capsule_name}",
        board.module, board.sku, board.fab, board.is_super, board.is_nanoe8gb
    );

    let Some(capsule_file) = find_capsule(Path::new(CAPSULE_SEARCH_DIR), capsule_name) else {
        // Update needed but the shipped capsule is missing: a packaging bug, not a skip.
        eprintln!("Capsule {capsule_name} not found under {CAPSULE_SEARCH_DIR}.");
        return Ok(ExitCode::FAILURE);
    };
    println!("Selected capsule: {}", capsule_file.display());

    let Some(esp_dev) = find_esp() else {
        eprintln!("No ESP labeled {ESP_LABEL} found; cannot stage capsule.");
        return Ok(ExitCode::FAILURE);
    };
    println!("ESP partition: {esp_dev}");

    if opts.dry_run {
        println!(
            "[dry-run] would stage {capsule_name} to <ESP>/EFI/UpdateCapsule/ and set OsIndications."
        );
        return Ok(ExitCode::SUCCESS);
    }

    stage_capsule(&esp_dev, &capsule_file, capsule_name)?;
    set_os_indications(&os_indications)?;
    println!("OsIndications EFI variable set. QSPI update will be applied on next reboot.");
    Ok(ExitCode::SUCCESS)
}

/// Target version, from the installed bootloader package.
fn read_target_version() -> Option<u32> {
    let out = Command::new("dpkg-query")
        .args(["-W", "-f", "${Version}", BL_PACKAGE])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    version::parse_deb_version(raw).ok()
}

/// Current version, from the ESRT firmware entry.
fn read_current_version() -> Option<u32> {
    let raw = fs::read_to_string(ESRT_FW_VERSION).ok()?;
    version::parse_esrt_version(&raw).ok()
}

fn read_capsule_status() -> Option<CapsuleStatus> {
    let out = Command::new("nvbootctrl")
        .arg("dump-slots-info")
        .output()
        .ok()?;
    nvbootctrl::parse_capsule_status(&String::from_utf8_lossy(&out.stdout))
}

fn os_indications_pending(path: &Path) -> bool {
    fs::read(path).is_ok_and(|bytes| osindications::capsule_pending(&bytes))
}

fn read_board() -> Option<capsule::Board> {
    let ids = fs::read(DT_IDS).ok()?;
    let ids = String::from_utf8_lossy(&ids);
    capsule::parse_board(&ids, &read_compat_spec().unwrap_or_default())
}

/// NVIDIA's compat-spec board name from its UEFI variable. The efivar is 4
/// attribute bytes then the ASCII string, as the firmware reads it. Absent or
/// unreadable yields the base (non-super, non-nanoe8gb) capsule, like the
/// firmware's own default.
fn read_compat_spec() -> Option<String> {
    let bytes = fs::read(Path::new(EFIVARS_DIR).join(COMPAT_SPEC_VAR)).ok()?;
    let text = String::from_utf8_lossy(bytes.get(4..)?);
    Some(text.trim_end_matches('\0').trim().to_string())
}

/// First match under `dir` by sorted path.
fn find_capsule(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut matches = Vec::new();
    collect_files(dir, name, &mut matches);
    matches.sort();
    matches.into_iter().next()
}

fn collect_files(dir: &Path, name: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_files(&entry.path(), name, out);
        } else if entry.file_name() == name {
            out.push(entry.path());
        }
    }
}

fn find_esp() -> Option<String> {
    let out = Command::new("blkid")
        .args(["-L", ESP_LABEL])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dev = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!dev.is_empty()).then_some(dev)
}

/// Mount the ESP transiently (torn down on drop) and stage the capsule.
fn stage_capsule(esp_dev: &str, capsule_file: &Path, capsule_name: &str) -> Result<()> {
    let esp = EspMount::mount(esp_dev)?;
    let capsule_dir = esp.path.join("EFI/UpdateCapsule");
    fs::create_dir_all(&capsule_dir).context("creating EFI/UpdateCapsule on the ESP")?;
    let dest = capsule_dir.join(capsule_name);
    fs::copy(capsule_file, &dest)
        .with_context(|| format!("copying {} to the ESP", capsule_file.display()))?;
    // FAT has no journal, so flush data and directory entries before reboot.
    rustix::fs::sync();
    println!("Capsule staged at <ESP>/EFI/UpdateCapsule/{capsule_name}");
    Ok(())
}

/// ESP mount that unmounts and removes its mountpoint on drop.
struct EspMount {
    path: PathBuf,
}

impl EspMount {
    fn mount(esp_dev: &str) -> Result<Self> {
        let path = PathBuf::from(format!("/run/qspi-esp.{}", std::process::id()));
        fs::create_dir_all(&path).context("creating ESP mountpoint")?;
        mount(
            esp_dev,
            &path,
            "vfat",
            MountFlags::NOSUID | MountFlags::NOEXEC,
            c"fmask=0177,dmask=0077",
        )
        .with_context(|| format!("mounting {esp_dev} at {}", path.display()))?;
        Ok(Self { path })
    }
}

impl Drop for EspMount {
    fn drop(&mut self) {
        let _ = unmount(&self.path, UnmountFlags::empty());
        let _ = fs::remove_dir(&self.path);
    }
}

/// Write `OsIndications` to request capsule processing next boot. efivarfs marks
/// existing variables immutable, so clear that first (a new one has nothing to
/// clear, hence best effort).
fn set_os_indications(path: &Path) -> Result<()> {
    let _ = clear_immutable(path);
    fs::write(path, osindications::REQUEST_CAPSULE)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Clear the immutable flag so the efivar can be written.
fn clear_immutable(path: &Path) -> Result<()> {
    let file = fs::File::open(path)?;
    let flags = ioctl_getflags(&file)?;
    if flags.contains(IFlags::IMMUTABLE) {
        let mut cleared = flags;
        cleared.remove(IFlags::IMMUTABLE);
        ioctl_setflags(&file, cleared)?;
    }
    Ok(())
}
