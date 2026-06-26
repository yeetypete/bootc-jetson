//! Firmware version encoding.
//!
//! Versions are compared as the decimal value of `0xRRRRMMmm`
//! (Release, Major, minor), the same encoding the ESRT exposes.

/// Encode a `rel.maj.min` triple as the ESRT does, `(rel<<16)|(maj<<8)|min`.
#[must_use]
pub fn encode(rel: u32, maj: u32, min: u32) -> u32 {
    (rel << 16) | (maj << 8) | min
}

/// Parse the installed `nvidia-l4t-bootloader` deb version into the ESRT
/// encoding. Takes the part before the first `-` (e.g. `39.2.0` from
/// `39.2.0-20260...`) and encodes its dotted fields.
///
/// # Errors
/// Fails if the version lacks three dotted numeric fields.
pub fn parse_deb_version(installed: &str) -> anyhow::Result<u32> {
    let upstream = installed.split('-').next().unwrap_or("");
    let mut fields = upstream.split('.');
    let rel = field(fields.next(), installed)?;
    let maj = field(fields.next(), installed)?;
    let min = field(fields.next(), installed)?;
    Ok(encode(rel, maj, min))
}

/// Parse the ESRT `fw_version`, which is already the decimal encoding.
///
/// # Errors
/// Fails if the value is not a decimal number.
pub fn parse_esrt_version(raw: &str) -> anyhow::Result<u32> {
    raw.trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("ESRT fw_version is not a number: {raw:?}"))
}

fn field(value: Option<&str>, full: &str) -> anyhow::Result<u32> {
    value
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("malformed bootloader version: {full:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_like_the_esrt() {
        // 0x230301 = 35.3.1 = 2_294_529.
        assert_eq!(encode(35, 3, 1), 2_294_529);
        // 0x270200 = 39.2.0 = 2_556_416 (JetPack 7.2 / r39.2).
        assert_eq!(encode(39, 2, 0), 2_556_416);
    }

    #[test]
    fn parses_deb_version_dropping_build_suffix() {
        assert_eq!(
            parse_deb_version("39.2.0-20260101120000").unwrap(),
            2_556_416
        );
        assert_eq!(
            parse_deb_version("35.3.1-20230314154120").unwrap(),
            2_294_529
        );
    }

    #[test]
    fn parses_deb_version_without_suffix() {
        assert_eq!(parse_deb_version("39.2.0").unwrap(), 2_556_416);
    }

    #[test]
    fn rejects_malformed_deb_version() {
        assert!(parse_deb_version("39.2").is_err());
        assert!(parse_deb_version("").is_err());
        assert!(parse_deb_version("a.b.c").is_err());
    }

    #[test]
    fn parses_esrt_decimal() {
        assert_eq!(parse_esrt_version("2556416").unwrap(), 2_556_416);
        assert_eq!(parse_esrt_version("  2556416\n").unwrap(), 2_556_416);
        assert!(parse_esrt_version("0x270200").is_err());
    }
}
