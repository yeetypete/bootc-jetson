//! Discover the ESP by label, mount it transiently, and stage a capsule.
//!
//! On bootc/composefs the ESP is not mounted at runtime, so we find it by label
//! and mount it transiently.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use rustix::mount::{MountFlags, UnmountFlags, mount, unmount};

/// Label bootc assigns the ESP at install.
pub const LABEL: &str = "EFI-SYSTEM";

/// Find the ESP block device by its label.
#[must_use]
pub fn find() -> Option<String> {
    let out = Command::new("blkid").args(["-L", LABEL]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let dev = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!dev.is_empty()).then_some(dev)
}

/// Mount the ESP transiently and copy the capsule into `EFI/UpdateCapsule/`.
///
/// # Errors
/// Fails if the ESP cannot be mounted or the capsule cannot be copied.
pub fn stage(esp_dev: &str, capsule_file: &Path, capsule_name: &str) -> Result<()> {
    let esp = Mount::new(esp_dev)?;
    let capsule_dir = esp.path.join("EFI/UpdateCapsule");
    fs::create_dir_all(&capsule_dir).context("creating EFI/UpdateCapsule on the ESP")?;
    let dest = capsule_dir.join(capsule_name);
    fs::copy(capsule_file, &dest)
        .with_context(|| format!("copying {} to the ESP", capsule_file.display()))?;
    // FAT has no journal, so flush data and directory entries before reboot.
    rustix::fs::sync();
    Ok(())
}

/// ESP mount that unmounts and removes its mountpoint on drop.
struct Mount {
    path: PathBuf,
}

impl Mount {
    fn new(esp_dev: &str) -> Result<Self> {
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

impl Drop for Mount {
    fn drop(&mut self) {
        let _ = unmount(&self.path, UnmountFlags::empty());
        let _ = fs::remove_dir(&self.path);
    }
}
