//! The `OsIndications` EFI variable used to request capsule-on-disk processing.
//!
//! efivarfs stores the variable as 4 little-endian attribute bytes followed by
//! the 8-byte little-endian value. Bit 2 (0x04) of the value is
//! `EFI_OS_INDICATIONS_FILE_CAPSULE_DELIVERY_SUPPORTED`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rustix::fs::{IFlags, ioctl_getflags, ioctl_setflags};

/// EFI global variable GUID, the namespace `OsIndications` lives in.
pub const EFI_GLOBAL_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

const EFIVARS_DIR: &str = "/sys/firmware/efi/efivars";

/// `OsIndications-<guid>` path under efivarfs.
fn path() -> PathBuf {
    Path::new(EFIVARS_DIR).join(format!("OsIndications-{EFI_GLOBAL_GUID}"))
}

/// True if a capsule-on-disk request from a prior boot is still pending, i.e.
/// UEFI has not consumed it.
#[must_use]
pub fn is_pending() -> bool {
    std::fs::read(path()).is_ok_and(|bytes| is_capsule_pending(&bytes))
}

/// Request capsule processing on next boot. efivarfs marks existing variables
/// immutable, so clear that first (a new one has nothing to clear, hence best
/// effort).
///
/// # Errors
/// Fails if the variable cannot be written.
pub fn request() -> Result<()> {
    let path = path();
    let _ = clear_immutable(&path);
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

/// True if the capsule-delivery bit is still set, i.e. a request is pending.
/// UEFI clears it once it consumes the request, so a still-set bit on a later
/// boot means a staged capsule was never processed.
#[must_use]
pub fn is_capsule_pending(bytes: &[u8]) -> bool {
    // Bit 2 lives in the value's low byte, at offset 4 (after the attributes).
    bytes.get(4).is_some_and(|b| b & 0x04 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_payload_is_pending() {
        assert!(is_capsule_pending(&REQUEST_CAPSULE));
    }

    #[test]
    fn cleared_value_is_not_pending() {
        let cleared = [0x07, 0, 0, 0, 0x00, 0, 0, 0, 0, 0, 0, 0];
        assert!(!is_capsule_pending(&cleared));
    }

    #[test]
    fn other_bits_do_not_count() {
        let other = [0x07, 0, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0];
        assert!(!is_capsule_pending(&other));
    }

    #[test]
    fn short_or_empty_is_not_pending() {
        assert!(!is_capsule_pending(&[]));
        assert!(!is_capsule_pending(&[0x07, 0, 0, 0]));
    }
}
