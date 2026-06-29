# jetson-tools

Rust workspace for Jetson host-side utilities. These utilities work across
Jetson bootc image variants (e.g. Orin, Thor). They are distributed together
with the jetson bootc images.

## Crates

- [`jetson-qspi-update`](crates/jetson-qspi-update): stages a QSPI firmware
  update via UEFI capsule-on-disk when the running firmware is older than the
  version shipped in the bootc image.
