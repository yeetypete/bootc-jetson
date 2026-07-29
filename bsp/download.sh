#!/usr/bin/env bash

# Download and extract a stock NVIDIA L4T BSP into bsp/l4t/<version>.
#
# Usage: download.sh [l4t-version]

set -euo pipefail

version="${1:-39.2.0}"
root="$(cd "$(dirname "$0")" && pwd)/l4t"

archive="Jetson_Linux_r${version}_aarch64.tbz2"
# 39.2.0 -> .../l4t/r39_release_v2.0/release/Jetson_Linux_r39.2.0_aarch64.tbz2
url="https://developer.nvidia.com/downloads/embedded/l4t/r${version%%.*}_release_v${version#*.}/release/${archive}"
tree="${root}/${version}/Linux_for_Tegra"

mkdir -p "${root}"

if [ ! -f "${root}/${archive}" ]; then
    echo "Downloading L4T r${version}..." >&2
    curl -fL --progress-bar -o "${root}/${archive}" "${url}"
fi

if [ ! -d "${tree}" ]; then
    echo "Extracting L4T r${version}..." >&2
    mkdir -p "${root}/${version}"
    tar xf "${root}/${archive}" -C "${root}/${version}"
fi

echo "${tree}"
