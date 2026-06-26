//! The `OsIndications` EFI variable used to request capsule-on-disk processing.
//!
//! efivarfs stores the variable as 4 little-endian attribute bytes followed by
//! the 8-byte little-endian value. Bit 2 (0x04) of the value is
//! `EFI_OS_INDICATIONS_FILE_CAPSULE_DELIVERY_SUPPORTED`.

/// EFI global variable GUID, the namespace `OsIndications` lives in.
pub const EFI_GLOBAL_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

/// `OsIndications-<guid>` filename under efivarfs.
#[must_use]
pub fn var_name() -> String {
    format!("OsIndications-{EFI_GLOBAL_GUID}")
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
pub fn capsule_pending(bytes: &[u8]) -> bool {
    // Bit 2 lives in the value's low byte, at offset 4 (after the attributes).
    bytes.get(4).is_some_and(|b| b & 0x04 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_payload_is_pending() {
        assert!(capsule_pending(&REQUEST_CAPSULE));
    }

    #[test]
    fn cleared_value_is_not_pending() {
        let cleared = [0x07, 0, 0, 0, 0x00, 0, 0, 0, 0, 0, 0, 0];
        assert!(!capsule_pending(&cleared));
    }

    #[test]
    fn other_bits_do_not_count() {
        let other = [0x07, 0, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0];
        assert!(!capsule_pending(&other));
    }

    #[test]
    fn short_or_empty_is_not_pending() {
        assert!(!capsule_pending(&[]));
        assert!(!capsule_pending(&[0x07, 0, 0, 0]));
    }
}
