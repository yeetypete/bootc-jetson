//! Firmware version comparison.
//!
//! The ESRT exposes the running firmware as the decimal of a packed
//! `0xRRRRMMmm` (Release, Major, minor) `u32`. The bootloader deb carries the
//! same `rel.maj.min` triple. We decode both to a [`Version`] and compare them
//! field by field.

use std::fmt;
use std::process::Command;

use anyhow::{Context, Result};

/// Deb package that ships the QSPI bootloader firmware.
pub const BL_PACKAGE: &str = "nvidia-l4t-bootloader";
const ESRT_FW_VERSION: &str = "/sys/firmware/efi/esrt/entries/entry0/fw_version";

/// A firmware version as a `rel.maj.min` triple. Ordering is field by field
/// (release, then major, then minor), matching firmware version precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub rel: u32,
    pub maj: u32,
    pub min: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.rel, self.maj, self.min)
    }
}

impl Version {
    /// Decode the ESRT packed encoding `0xRRRRMMmm`.
    #[must_use]
    pub fn from_esrt_encoded(packed: u32) -> Self {
        Self {
            rel: packed >> 16,
            maj: (packed >> 8) & 0xff,
            min: packed & 0xff,
        }
    }
}

/// Target version, from the installed bootloader package.
#[must_use]
pub fn read_target() -> Option<Version> {
    let out = Command::new("dpkg-query")
        .args(["-W", "-f", "${Version}", BL_PACKAGE])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    parse_deb_version(raw.trim()).ok()
}

/// Current version, from the ESRT firmware entry.
#[must_use]
pub fn read_current() -> Option<Version> {
    let raw = std::fs::read_to_string(ESRT_FW_VERSION).ok()?;
    parse_esrt_version(&raw).ok()
}

/// Parse the installed `nvidia-l4t-bootloader` deb version. Takes the upstream
/// part before the debian revision (e.g. `39.2.0` from `39.2.0-20260...`) and
/// parses it as a semantic version.
///
/// # Errors
/// Fails if the upstream version is not a `rel.maj.min` triple.
pub fn parse_deb_version(installed: &str) -> Result<Version> {
    let upstream = installed.split('-').next().unwrap_or_default();
    let v = semver::Version::parse(upstream)
        .with_context(|| format!("malformed bootloader version: {installed:?}"))?;
    Ok(Version {
        rel: u32::try_from(v.major).context("release field too large")?,
        maj: u32::try_from(v.minor).context("major field too large")?,
        min: u32::try_from(v.patch).context("minor field too large")?,
    })
}

/// Parse the ESRT `fw_version`, the packed `0xRRRRMMmm` encoding as a decimal.
///
/// # Errors
/// Fails if the value is not a decimal number.
pub fn parse_esrt_version(raw: &str) -> Result<Version> {
    let packed: u32 = raw
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("ESRT fw_version is not a number: {raw:?}"))?;
    Ok(Version::from_esrt_encoded(packed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_esrt_encoding() {
        // 0x230301 = 35.3.1 = 2_294_529.
        assert_eq!(
            Version::from_esrt_encoded(2_294_529),
            Version {
                rel: 35,
                maj: 3,
                min: 1
            }
        );
        // 0x270200 = 39.2.0 = 2_556_416 (JetPack 7.2 / r39.2).
        assert_eq!(
            Version::from_esrt_encoded(2_556_416),
            Version {
                rel: 39,
                maj: 2,
                min: 0
            }
        );
    }

    #[test]
    fn esrt_and_deb_agree() {
        // The ESRT decode and the deb parse must land on the same triple.
        assert_eq!(
            parse_esrt_version("2556416").unwrap(),
            parse_deb_version("39.2.0-20260601141651").unwrap()
        );
    }

    #[test]
    fn parses_deb_version_dropping_build_suffix() {
        assert_eq!(
            parse_deb_version("39.2.0-20260101120000").unwrap(),
            Version {
                rel: 39,
                maj: 2,
                min: 0
            }
        );
        assert_eq!(
            parse_deb_version("35.3.1-20230314154120").unwrap(),
            Version {
                rel: 35,
                maj: 3,
                min: 1
            }
        );
    }

    #[test]
    fn parses_deb_version_without_suffix() {
        assert_eq!(
            parse_deb_version("39.2.0").unwrap(),
            Version {
                rel: 39,
                maj: 2,
                min: 0
            }
        );
    }

    #[test]
    fn rejects_malformed_deb_version() {
        assert!(parse_deb_version("39.2").is_err());
        assert!(parse_deb_version("").is_err());
        assert!(parse_deb_version("a.b.c").is_err());
    }

    #[test]
    fn parses_esrt_decimal() {
        assert_eq!(
            parse_esrt_version("2556416").unwrap(),
            Version {
                rel: 39,
                maj: 2,
                min: 0
            }
        );
        assert_eq!(
            parse_esrt_version("  2556416\n").unwrap(),
            Version {
                rel: 39,
                maj: 2,
                min: 0
            }
        );
        assert!(parse_esrt_version("0x270200").is_err());
    }
}
