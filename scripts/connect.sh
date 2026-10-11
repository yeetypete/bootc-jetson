#!/usr/bin/env bash

# Connect to a Jetson's serial console over USB, chosen from a menu.

set -euo pipefail

devices=()
for dev in /dev/ttyACM* /dev/ttyUSB*; do
    [[ -e $dev ]] || continue
    model=$(udevadm info --query=property --property=ID_MODEL_FROM_DATABASE --value "$dev" 2>/dev/null)
    devices+=("$dev ${model:--}")
done
if [[ ${#devices[@]} -eq 0 ]]; then
    echo "No USB serial devices found." >&2
    exit 1
fi

PS3="Select the serial device: "
select entry in "${devices[@]}"; do
    [[ -n "${entry:-}" ]] && break
done
device=${entry%% *}

echo "Connecting to ${device} at 115200 baud. Quit with Ctrl-a x."
exec minicom -D "$device" -b 115200
