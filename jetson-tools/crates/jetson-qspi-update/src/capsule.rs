//! Capsule selection, mirroring edk2-nvidia's `SelectCapsuleFile`. Board
//! id/SKU/FAB come from the device tree `chosen/ids`, and the Super and
//! nanoe8gb sub-variants from the `TegraPlatformCompatSpec` UEFI variable.

/// Board identity, parsed from the device tree and the compat-spec variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    pub id: u32,
    pub sku: u32,
    pub fab: u32,
    pub is_super: bool,
    pub is_nanoe8gb: bool,
}

/// Module ids we know how to map to a capsule (Orin AGX, Orin Nano/NX, Thor AGX).
const KNOWN_IDS: [u32; 3] = [3701, 3767, 3834];

/// Board name the compat spec carries for the Orin Nano e 8GB, the only thing
/// that sets it apart from the base Orin Nano. Mirrors edk2-nvidia's
/// `BOARD_NAME_ORIN_NANOE8GB_DEVKIT`.
const NANOE8GB_BOARD_NAME: &str = "jetson-orin-nanoe8gb-devkit";

/// Parse board identity from `chosen/ids` and NVIDIA's compat-spec string.
/// `chosen/ids` holds NUL- or space-separated `<id>-<sku>-<fab>` tokens (one per
/// board), so we pick the first whose id is a known module. The compat spec
/// carries the board name we match for the Super and nanoe8gb sub-variants.
#[must_use]
pub fn parse_board(ids_raw: &str, compat_spec: &str) -> Option<Board> {
    let is_super = is_super(compat_spec);
    let is_nanoe8gb = compat_spec.contains(NANOE8GB_BOARD_NAME);
    ids_raw
        .split([' ', '\0', '\n', '\t'])
        .filter(|t| !t.is_empty())
        .find_map(|tok| {
            let mut parts = tok.split('-');
            let id: u32 = parts.next()?.parse().ok()?;
            if !KNOWN_IDS.contains(&id) {
                return None;
            }
            let sku: u32 = parts.next()?.parse().ok()?;
            let fab: u32 = parts.next()?.parse().ok()?;
            Some(Board {
                id,
                sku,
                fab,
                is_super,
                is_nanoe8gb,
            })
        })
}

/// Super power profile, mirroring edk2-nvidia `IsSuper`, a substring check for
/// "super" in the compat spec.
fn is_super(compat_spec: &str) -> bool {
    compat_spec.contains("super")
}

/// Select the capsule filename for a board, mirroring `SelectCapsuleFile`.
/// Returns `None` for a module id we have no mapping for.
#[must_use]
pub fn select_capsule(board: &Board) -> Option<&'static str> {
    Some(match board.id {
        3701 => {
            // AGX Orin. Super wins over the SKU/FAB default, matching the
            // firmware, which computes the default then lets a super profile
            // override it. Industrial (sku 8) is its own image, and an early
            // sku-0 board (FAB other than 300) takes the legacy capsule.
            if board.is_super {
                "TEGRA_BL_3701_agx_super.Cap"
            } else if board.sku == 8 {
                "TEGRA_BL_3701_agx_ind.Cap" // industrial
            } else if board.sku == 0 && board.fab != 300 {
                "TEGRA_BL_3701_000.Cap"
            } else {
                "TEGRA_BL_3701_agx.Cap"
            }
        }
        // Orin Nano / NX. nanoe8gb is its own image, set apart only by the
        // compat spec board name. Mirrors `GetOrinNanoCapsuleFileName`.
        3767 => match (board.is_nanoe8gb, board.is_super) {
            (true, true) => "TEGRA_BL_3767_nanoe8gb_super.Cap",
            (true, false) => "TEGRA_BL_3767_nanoe8gb.Cap",
            (false, true) => "TEGRA_BL_3767_super.Cap",
            (false, false) => "TEGRA_BL_3767.Cap",
        },
        3834 => "TEGRA_BL_3834_agx.Cap", // AGX Thor
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn select(ids: &str, compat: &str) -> Option<&'static str> {
        select_capsule(&parse_board(ids, compat)?)
    }

    #[test]
    fn orin_nano_super() {
        assert_eq!(
            select(
                "3767-0005-300 3768-0000-400",
                "3767-0005-300--1--jetson-orin-nano-devkit-super-"
            ),
            Some("TEGRA_BL_3767_super.Cap")
        );
    }

    #[test]
    fn orin_nano_nx_no_super() {
        assert_eq!(
            select(
                "3767-0000-500 3768-0000-400",
                "3767-0000-500--1--jetson-orin-nano-devkit-"
            ),
            Some("TEGRA_BL_3767.Cap")
        );
    }

    #[test]
    fn orin_nanoe8gb() {
        assert_eq!(
            select(
                "3767-0000-500 3768-0000-400",
                "3767-0000-500--1--jetson-orin-nanoe8gb-devkit-"
            ),
            Some("TEGRA_BL_3767_nanoe8gb.Cap")
        );
    }

    #[test]
    fn orin_nanoe8gb_super() {
        assert_eq!(
            select(
                "3767-0000-500 3768-0000-400",
                "3767-0000-500--1--jetson-orin-nanoe8gb-devkit-super-"
            ),
            Some("TEGRA_BL_3767_nanoe8gb_super.Cap")
        );
    }

    #[test]
    fn agx_orin_sku5() {
        assert_eq!(
            select(
                "3701-0005-400 3737-0000-500",
                "3701-0005-400--1--jetson-agx-orin-devkit-"
            ),
            Some("TEGRA_BL_3701_agx.Cap")
        );
    }

    #[test]
    fn agx_orin_super() {
        assert_eq!(
            select(
                "3701-0000-400 3737-0000-500",
                "3701-0000-400--1--jetson-agx-orin-devkit-super-"
            ),
            Some("TEGRA_BL_3701_agx_super.Cap")
        );
    }

    #[test]
    fn agx_orin_industrial() {
        assert_eq!(
            select(
                "3701-0008-400 3737-0000-500",
                "3701-0008-400--1--jetson-agx-orin-devkit-industrial-"
            ),
            Some("TEGRA_BL_3701_agx_ind.Cap")
        );
    }

    #[test]
    fn agx_orin_super_overrides_sku() {
        // Super wins over the SKU/FAB default, as the firmware orders it.
        assert_eq!(
            select(
                "3701-0008-400 3737-0000-500",
                "3701-0008-400--1--jetson-agx-orin-devkit-super-"
            ),
            Some("TEGRA_BL_3701_agx_super.Cap")
        );
    }

    #[test]
    fn agx_orin_sku0_fab300() {
        assert_eq!(
            select(
                "3701-0000-300 3737-0000-500",
                "3701-0000-300--1--jetson-agx-orin-devkit-"
            ),
            Some("TEGRA_BL_3701_agx.Cap")
        );
    }

    #[test]
    fn agx_orin_sku0_other_fab() {
        assert_eq!(
            select(
                "3701-0000-400 3737-0000-500",
                "3701-0000-400--1--jetson-agx-orin-devkit-"
            ),
            Some("TEGRA_BL_3701_000.Cap")
        );
    }

    #[test]
    fn agx_thor() {
        assert_eq!(
            select(
                "3834-0008-400 4071-0000-500",
                "3834-0008-400--1--jetson-agx-thor-devkit-"
            ),
            Some("TEGRA_BL_3834_agx.Cap")
        );
    }

    #[test]
    fn unknown_board() {
        assert_eq!(parse_board("9999-0000-000", "nvidia,whatever"), None);
        assert_eq!(select("9999-0000-000", "nvidia,whatever"), None);
    }

    #[test]
    fn picks_module_token_not_carrier() {
        // Carrier 3768/3737 can come first, so we must skip to the module.
        let b = parse_board("3737-0000-500 3701-0005-400", "x").unwrap();
        assert_eq!(b.id, 3701);
        assert_eq!(b.sku, 5);
    }

    #[test]
    fn handles_nul_separators() {
        // /proc/device-tree files are NUL-delimited with a trailing NUL.
        let b = parse_board("3767-0005-300\x003768-0000-400\x00", "x-super").unwrap();
        assert_eq!(
            b,
            Board {
                id: 3767,
                sku: 5,
                fab: 300,
                is_super: true,
                is_nanoe8gb: false,
            }
        );
    }
}
