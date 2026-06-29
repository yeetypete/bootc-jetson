//! The `OsIndications` EFI variable used to request capsule-on-disk processing.
//!
//! efivarfs stores the variable as 4 little-endian attribute bytes followed by
//! the 8-byte little-endian value. Bit 2 (0x04) of the value is
//! `EFI_OS_INDICATIONS_FILE_CAPSULE_DELIVERY_SUPPORTED`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rustix::fs::{IFlags, ioctl_getflags, ioctl_setflags};
use tracing::debug;

/// EFI global variable GUID, the namespace `OsIndications` lives in.
pub const EFI_GLOBAL_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

const EFIVARS_DIR: &str = "/sys/firmware/efi/efivars";

/// `OsIndications-<guid>` path under efivarfs.
fn path() -> PathBuf {
    Path::new(EFIVARS_DIR).join(format!("OsIndications-{EFI_GLOBAL_GUID}"))
}

/// Request capsule processing on next boot. efivarfs marks existing variables
/// immutable, so clear that first.
///
/// # Errors
/// Fails if the variable cannot be written.
pub fn request() -> Result<()> {
    let path = path();
    if let Err(e) = clear_immutable(&path) {
        debug!("clearing immutable flag on {} failed: {e}", path.display());
    }
    std::fs::write(&path, REQUEST_CAPSULE)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Clear the immutable flag so the efivar can be written.
fn clear_immutable(path: &Path) -> Result<()> {
    let file = std::fs::File::open(path)?;
    let flags = ioctl_getflags(&file)?;
    if flags.contains(IFlags::IMMUTABLE) {
        let mut cleared = flags;
        cleared.remove(IFlags::IMMUTABLE);
        ioctl_setflags(&file, cleared)?;
    }
    Ok(())
}

/// Payload requesting capsule-on-disk next boot (attributes `NV|BS|RT` then a
/// value with only bit 2 set).
pub const REQUEST_CAPSULE: [u8; 12] = [
    0x07, 0x00, 0x00, 0x00, // attributes NON_VOLATILE | BOOTSERVICE | RUNTIME
    0x04, 0x00, 0x00, 0x00, // value low 4 bytes, FILE_CAPSULE_DELIVERY_SUPPORTED
    0x00, 0x00, 0x00, 0x00, // value high 4 bytes
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_payload_sets_only_the_capsule_bit() {
        // Bit 2 of the value (byte 4, after the 4 attribute bytes) requests
        // capsule-on-disk; no other value bit is set.
        assert_eq!(REQUEST_CAPSULE[4], 0x04);
        assert_eq!(&REQUEST_CAPSULE[5..], &[0u8; 7]);
    }
}
