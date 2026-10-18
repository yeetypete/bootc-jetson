# bootc-jetson developer tasks.

# Image repository for the built images.
repository := "docker.io/yeetypete/bootc-jetson"
# JetPack release these images target.
jetpack := "7.2"
# Variant to build, e.g. `just variant=thor dist`. Each name is the variant's build dir.
variant := "orin"

# The rolling tag installed systems follow with `bootc upgrade`.
tag := variant + "-jp" + jetpack
reference := repository + ":" + tag
# Local image of the rootfs stage, which chunkah rechunks.
rootfs := "localhost/bootc-jetson-" + variant + "-rootfs"
# chunkah is used to split the rootfs into a layer per group of packages.
chunkah_image := "quay.io/coreos/chunkah:v0.7.1@sha256:71e593421c5de7dcc7c8c5c50e98d3bac572055a76f399f61a491d123c20a523"
disk_name := "bootc-jetson-" + variant
disk_size := "10G"
oci_archive := "image.oci"
# UEFI firmware for `vm`, from the qemu-efi-aarch64 package.
aavmf := "/usr/share/AAVMF"

# List available recipes.
default:
    @just --list

# Check formatting and lint the jetson-tools Rust workspace.
check:
    cd jetson-tools && cargo fmt --check
    cd jetson-tools && cargo clippy --all-targets --locked -- -D warnings

# Run the jetson-tools Rust workspace tests.
test:
    cd jetson-tools && cargo test --locked

# Build the rootfs stage, rechunk it with chunkah, and build the image from it.
build *args:
    podman build --platform linux/arm64 --target rootfs --tag {{ rootfs }} {{ args }} {{ variant }}
    rm -rf {{ variant }}/chunked
    podman run --rm \
        --mount type=image,src={{ rootfs }},dst=/chunkah \
        --volume "$PWD/{{ variant }}:/srv:z" \
        --env CHUNKAH_CONFIG_STR="$(podman image inspect {{ rootfs }})" \
        {{ chunkah_image }} build \
            --max-layers 128 \
            --source-date-epoch 0 \
            --output oci:chunked
    podman build --platform linux/arm64 \
        --build-context chunked=container-image://"$(podman pull --quiet oci:{{ variant }}/chunked)" \
        --tag {{ reference }} {{ args }} {{ variant }}

# Save the image as an OCI archive for `disk`.
save:
    # bootc runs as root and cannot read the rootless podman store.
    rm -f {{ oci_archive }}
    podman save --quiet --format oci-archive --output {{ oci_archive }} {{ reference }}

# Push the image tagged <tag>-<short_sha>, and for a release also <tag>-<version> and <tag>.
push short_sha version="":
    #!/usr/bin/env bash
    set -euo pipefail
    # The short sha tag is pushed on every build.
    tags=("{{ tag }}-{{ short_sha }}")
    # Releases also push the version tag and move the rolling tag, which
    # `bootc upgrade` follows.
    if [ -n "{{ version }}" ]; then
        tags+=("{{ tag }}-{{ version }}" "{{ tag }}")
    fi
    for t in "${tags[@]}"; do
        podman push --compression-format zstd --force-compression \
            {{ reference }} "{{ repository }}:$t"
    done

# Install an image into a flashable raw disk image via bootc install to-disk.
disk imgref=("oci-archive:" + oci_archive):
    truncate -s {{ disk_size }} {{ disk_name }}.img
    sudo podman run \
        --rm --privileged --pid=host \
        --security-opt label=disable \
        -v /dev:/dev \
        -v "$PWD:/output" \
        -w /output \
        {{ imgref }} \
        bootc install to-disk \
            --source-imgref {{ imgref }} \
            --target-imgref {{ reference }} \
            --composefs-backend \
            --via-loopback /output/{{ disk_name }}.img

# Compress and checksum the disk image for distribution.
compress:
    zstd -T0 -v -o {{ disk_name }}.img.zst {{ disk_name }}.img
    sha256sum {{ disk_name }}.img.zst > {{ disk_name }}.img.zst.sha256

# Install, compress, and checksum the disk image.
dist imgref=("oci-archive:" + oci_archive): (disk imgref) compress

# Download the newest CI disk image built from a branch, by default the current one.
download branch=`git branch --show-current`:
    scripts/download.sh {{ variant }} {{ branch }}

# Boot a disk image in an emulated arm64 VM. Quit with Ctrl-a x, SSH in on port 2222.
vm file=(disk_name + ".img"):
    qemu-system-aarch64 \
        -snapshot \
        -machine virt \
        -cpu cortex-a72 \
        -smp 4 \
        -m 4G \
        -drive if=pflash,format=raw,readonly=on,file={{ aavmf }}/AAVMF_CODE.fd \
        -drive if=pflash,format=raw,file={{ aavmf }}/AAVMF_VARS.fd \
        -drive if=virtio,format=raw,file={{ file }} \
        -nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:2222-:22 \
        -nographic

# Write a disk image to an external storage device.
flash file=(disk_name + ".img"):
    scripts/flash.sh {{ file }}

# Connect to a Jetson's serial console over USB.
connect:
    scripts/connect.sh

# Remove generated disk images, the OCI archive and the chunkah output.
clean:
    rm -rf {{ disk_name }}*.img* {{ oci_archive }} {{ variant }}/chunked
