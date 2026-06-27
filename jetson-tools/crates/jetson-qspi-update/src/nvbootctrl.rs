//! Parsing of `nvbootctrl dump-slots-info`, used to tell whether a previously
//! staged capsule was applied or failed.

use std::process::Command;

/// Read the capsule update status by running `nvbootctrl dump-slots-info`.
#[must_use]
pub fn read_capsule_status() -> Option<CapsuleStatus> {
    let out = Command::new("nvbootctrl")
        .arg("dump-slots-info")
        .output()
        .ok()?;
    parse_capsule_status(&String::from_utf8_lossy(&out.stdout))
}

/// Capsule update status reported by nvbootctrl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapsuleStatus {
    /// 0 = no capsule update in progress.
    None,
    /// 1 = last capsule update succeeded.
    Success,
    /// 2 = installed, but the new firmware failed to boot.
    BootFailed,
    /// 3 = the capsule install itself failed.
    InstallFailed,
    /// Any other / unrecognized code.
    Other(u32),
}

impl CapsuleStatus {
    fn from_code(code: u32) -> Self {
        match code {
            0 => Self::None,
            1 => Self::Success,
            2 => Self::BootFailed,
            3 => Self::InstallFailed,
            n => Self::Other(n),
        }
    }

    /// True if a capsule was attempted and the update did not succeed.
    #[must_use]
    pub fn is_failure(self) -> bool {
        matches!(self, Self::BootFailed | Self::InstallFailed)
    }
}

/// Extract the capsule status code from `dump-slots-info` output. The status
/// line looks like `Capsule update status: 0`, so we take the trailing number.
/// Returns `None` if the line is absent (older firmware) or unparseable.
pub fn parse_capsule_status(output: &str) -> Option<CapsuleStatus> {
    output
        .lines()
        .find(|l| l.contains("Capsule update status"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|tok| tok.parse().ok())
        .map(CapsuleStatus::from_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Current version: 39.2.0
Capsule update status: 0
Current bootloader slot: A
";

    #[test]
    fn parses_status_line() {
        assert_eq!(parse_capsule_status(SAMPLE), Some(CapsuleStatus::None));
        assert_eq!(
            parse_capsule_status("Capsule update status: 1"),
            Some(CapsuleStatus::Success)
        );
        assert_eq!(
            parse_capsule_status("Capsule update status: 2"),
            Some(CapsuleStatus::BootFailed)
        );
        assert_eq!(
            parse_capsule_status("Capsule update status: 3"),
            Some(CapsuleStatus::InstallFailed)
        );
    }

    #[test]
    fn missing_line_is_none() {
        assert_eq!(parse_capsule_status("Current version: 39.2.0"), None);
        assert_eq!(parse_capsule_status(""), None);
    }

    #[test]
    fn failure_classification() {
        assert!(!CapsuleStatus::None.is_failure());
        assert!(!CapsuleStatus::Success.is_failure());
        assert!(CapsuleStatus::BootFailed.is_failure());
        assert!(CapsuleStatus::InstallFailed.is_failure());
        assert!(!CapsuleStatus::Other(9).is_failure());
    }
}
