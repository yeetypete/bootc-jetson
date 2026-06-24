# bootc-jetson

A [bootc](https://bootc-dev.github.io/bootc/) base image for NVIDIA Jetson Orin,
running [JetPack 7.2](https://developer.nvidia.com/embedded/jetpack) (Jetson
Linux r39.2).

> [!WARNING]
> This repository is intended as a **reference example** of running `bootc` on
> Jetson hardware, not as a base image that other projects consume directly.
> Fork it and adapt the `Dockerfile` and `rootfs/` to your own needs rather than
> depending on the published tags.

Support is currently limited to:

- **JetPack 7.2 only** (Jetson Linux r39.2).
- **Jetson Orin only** (AGX Orin, Orin NX, Orin Nano). Jetson Thor support is planned
  for a later release.

## What's in the image

- `bootc` with the [composefs backend](https://bootc.dev/bootc/experimental-composefs.html)
  enabled for image storage and deployment, a read-only root filesystem, and
  [transient `/etc`](https://bootc.dev/bootc/filesystem.html#enabling-transient-etc)
  (changes to `/etc` reset on reboot).
- Ubuntu 24.04 with the NVIDIA Jetson L4T packages for JetPack 7.2 (CUDA,
  TensorRT, multimedia, and the GPU/display stack).
- A custom build of the `nvidia-l4t-kernel` (6.8 Tegra) with `EROFS` and
  `FS_VERITY` enabled, as required by `bootc`'s composefs backend.
- A `systemd-boot` + `systemd-networkd`/`resolved`/`timesyncd` base with SSH
  enabled.
- An `ubuntu` user (password `ubuntu`, passwordless `sudo`).

## Requirements

- A Jetson Orin developer kit or module, with its boot firmware already at
  JetPack 7.2 (see [Provisioning a Jetson Orin](#provisioning-a-jetson-orin)).
- A Linux host with [`just`](https://github.com/casey/just) and `zstd` to flash
  a release build (see [Option 1](#option-1-flash-a-release-build)).
- Only for [building locally](#option-2-build-locally): an `arm64` host with
  [Docker](https://docs.docker.com/) installed.

## Provisioning a Jetson Orin

> [!IMPORTANT]
> The bootc image only manages the root filesystem on disk. The device's boot
> firmware must already be at a JetPack 7.2 (Jetson Linux r39.2) level. Follow
> the official developer kit guide for your board for flashing instructions:
>
> - [Jetson AGX Orin Developer Kit - BSP Installation](https://docs.nvidia.com/jetson/agx-orin-devkit/user-guide/latest/setup_bsp.html)
> - [Jetson Orin Nano Developer Kit - BSP Setup](https://docs.nvidia.com/jetson/orin-nano-devkit/user-guide/latest/setup_bsp.html)

### Option 1: flash a release build

Download the latest `bootc-jetson-orin-<version>.img.zst` and its matching
`bootc-jetson-orin-<version>.img.zst.sha256` from the
[GitHub releases](https://github.com/yeetypete/bootc-jetson/releases) page.
Verify and decompress it, then write it to your Jetson's root filesystem
device (e.g. an SSD):

```bash
sha256sum -c bootc-jetson-orin*.img.zst.sha256
zstd -d bootc-jetson-orin*.img.zst -o bootc-jetson-orin.img

just flash
```

`just flash` will prompt you to pick the target device from a menu and will then
write the image.

### Option 2: build locally

> [!NOTE]
> Building locally requires an `arm64` host.

```bash
just build  # Build the Jetson bootc image (OCI archive).
just disk   # Install the image into a loopback raw disk image.
just flash  # Write the disk image to an SSD.
```

Once booted, the system updates transactionally with `bootc upgrade`, which
pulls a newer image and stages it as a new deployment you can roll back to if
needed. See the [`bootc` upgrade docs](https://bootc-dev.github.io/bootc/upgrades.html).

## License

`bootc-jetson` is released under the
[MIT License](LICENSE).
