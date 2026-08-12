#!/usr/bin/env bash

# Install the host packages needed to flash a Jetson.
#
# This mirrors tools/l4t_flash_prerequisites.sh from the L4T BSP, which fails on
# Ubuntu 26.04.
#
# Usage: prereqs.sh

set -euo pipefail

packages=(
    abootimg binfmt-support binutils cpio cpp device-tree-compiler dosfstools
    e2fsprogs file gdisk iproute2 iputils-ping lbzip2 libxml2-utils lz4
    netcat-openbsd nfs-kernel-server openssl parted python3-usb python3-yaml
    rsync sshpass udev usbutils uuid-runtime whois xmlstarlet xxd zlib1g zstd
)

# A candidate other than "(none)" means the package is installable here.
if apt-cache policy qemu-user-static 2>/dev/null | grep -q 'Candidate: [^(]'; then
    packages+=(qemu-user-static)
else
    packages+=(qemu-user qemu-user-binfmt)
fi

sudo apt-get update
sudo apt-get install -y "${packages[@]}"
