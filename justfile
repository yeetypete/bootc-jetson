# bootc-jetson developer tasks.

# Image repository for the built images.
image := "docker.io/yeetypete/bootc-jetson"
# JetPack release these images target.
jetpack := "7.2"
# Variant to build, e.g. `just variant=thor dist`. Each name is the variant's build dir.
variant := "orin"

# The rolling tag installed systems follow with `bootc upgrade`.
tag := variant + "-jp" + jetpack
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

# Build the bootc container image, and save it as an OCI archive for `disk`.
build *args:
    podman build --platform linux/arm64 --tag {{ image }}:{{ tag }} {{ args }} {{ variant }}
    # bootc runs as root and cannot read the rootless podman store.
    rm -f {{ oci_archive }}
    podman save --quiet --format oci-archive --output {{ oci_archive }} {{ image }}:{{ tag }}

# Install an image into a flashable raw disk image via bootc install to-disk.
disk source=("oci-archive:" + oci_archive):
    # `source` is a containers-transports(5) reference, e.g. docker://<image>:<tag>.
    truncate -s {{ disk_size }} {{ disk_name }}.img
    sudo podman run \
        --rm --privileged --pid=host \
        --security-opt label=disable \
        -v /dev:/dev \
        -v "$PWD:/output" \
        -w /output \
        {{ source }} \
        bootc install to-disk \
            --source-imgref {{ source }} \
            --target-imgref {{ image }}:{{ tag }} \
            --composefs-backend \
            --via-loopback /output/{{ disk_name }}.img

# Compress and checksum the disk image for distribution.
compress:
    zstd -T0 -v -o {{ disk_name }}.img.zst {{ disk_name }}.img
    sha256sum {{ disk_name }}.img.zst > {{ disk_name }}.img.zst.sha256

# Install, compress, and checksum the disk image.
dist source=("oci-archive:" + oci_archive): (disk source) compress

# Download the newest CI disk image built from a branch, by default the current one.
download branch=`git branch --show-current`:
    scripts/download.sh {{ variant }} {{ branch }}

# Boot a disk image in an emulated arm64 VM. Quit with Ctrl-a x, SSH in on port 2222.
vm image=(disk_name + ".img"):
    qemu-system-aarch64 \
        -snapshot \
        -machine virt \
        -cpu cortex-a72 \
        -smp 4 \
        -m 4G \
        -drive if=pflash,format=raw,readonly=on,file={{ aavmf }}/AAVMF_CODE.fd \
        -drive if=pflash,format=raw,file={{ aavmf }}/AAVMF_VARS.fd \
        -drive if=virtio,format=raw,file={{ image }} \
        -nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:2222-:22 \
        -nographic

# Write a disk image to an external storage device.
flash image=(disk_name + ".img"):
    scripts/flash.sh {{ image }}

# Remove generated disk images and the OCI archive.
clean:
    rm -f {{ disk_name }}*.img {{ disk_name }}*.img.zst {{ disk_name }}*.img.zst.sha256 {{ oci_archive }}
