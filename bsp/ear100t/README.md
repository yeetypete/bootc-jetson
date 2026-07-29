# EverFocus EAR100T board support

The bootc image manages the root filesystem. The EAR100T's board configuration
lives in QSPI boot firmware, which is flashed separately with NVIDIA's L4T BSP.
`apply.sh` generates that firmware delta.

## Requirements

- An Ubuntu x86_64 host, connected to the board's USB recovery port.
- Run the following to install all the prerequisites for flashing:

  ```bash
  sudo "$(just bsp-download)/tools/l4t_flash_prerequisites.sh"
  ```

## Flashing

`just bsp-flash` downloads the BSP, applies the delta and flashes, so the
individual steps below are only needed to run one on its own.

Write the delta into the BSP tree:

```bash
just bsp-patch
```

Put the board into USB recovery mode. From a running system that is
`sudo reboot --force forced-recovery`, otherwise see the EverFocus
documentation. Confirm the host sees it, where `0955:7026` is an AGX Thor in
recovery mode:

```bash
lsusb | grep 0955:7026
```

Flash the boot firmware. This writes QSPI only and does not touch the root filesystem

```bash
just bsp-flash
```

Then install the bootc image to the NVMe as described in [README.md](../../README.md).

## NVIDIA references

- [Quick Start][qs] and [Flashing Support][fs], Jetson Linux Developer Guide
- [BSP Setup][thor], Jetson AGX Thor Developer Kit User Guide
- [Jetson Linux Archive][archive], for release downloads

[qs]: https://docs.nvidia.com/jetson/archives/r39.2/DeveloperGuide/IN/QuickStart.html
[fs]: https://docs.nvidia.com/jetson/archives/r39.2/DeveloperGuide/SD/FlashingSupport.html
[thor]: https://docs.nvidia.com/jetson/agx-thor-devkit/user-guide/latest/setup_bsp.html
[archive]: https://developer.nvidia.com/embedded/jetson-linux-archive
