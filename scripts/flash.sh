#!/usr/bin/env bash

# Write a disk image to a USB-attached SSD, chosen from a menu.

set -euo pipefail

DISK_IMAGE="${1:?Usage: flash.sh <disk-image>}"

if [[ ! -f "$DISK_IMAGE" ]]; then
    echo "Error: ${DISK_IMAGE} not found" >&2
    exit 1
fi

mapfile -t devices < <(lsblk -dn -o PATH,SIZE,MODEL,TRAN | awk '$NF == "usb"')
if [[ ${#devices[@]} -eq 0 ]]; then
    echo "No USB-attached block devices found." >&2
    exit 1
fi

PS3="Select the target device: "
select entry in "${devices[@]}"; do
    [[ -n "${entry:-}" ]] && break
done
device=${entry%% *}

echo "Write ${DISK_IMAGE} to ${entry}? All data on the device will be destroyed."
read -rp "[y/N] " reply
[[ $reply == [Yy] ]] || exit 1

sudo dd if="$DISK_IMAGE" of="$device" bs=4M status=progress conv=fsync
