# bootc-jetson developer tasks.

# Image repository for the built images.
image := "yeetypete/bootc-jetson"
# Version tag for the built images.
version := "0.0.0"
# Git commit SHA for image labels.
revision := `git rev-parse HEAD 2>/dev/null || echo ""`

# Build context and tags for the Jetson Orin variant.
context := "orin"
tag := "orin-jp7.2"

# Disk image settings (see .github/image-variants.json).
disk_name := "bootc-jetson-orin"
disk_size := "10G"

# List available recipes.
default:
    @just --list

# Build the Jetson Orin bootc container image.
build:
    podman build \
        --platform linux/arm64 \
        --label org.opencontainers.image.version={{ version }} \
        --label org.opencontainers.image.revision={{ revision }} \
        -f {{ context }}/Dockerfile \
        -t {{ image }}:{{ tag }} \
        -t {{ image }}:{{ tag }}-{{ version }} \
        {{ context }}

# Convert the built image into a flashable raw disk image via bootc install to-disk.
disk:
    truncate -s {{ disk_size }} {{ disk_name }}.img
    sudo podman run \
        --rm --privileged --pid=host \
        --security-opt label=type:unconfined_t \
        -v /dev:/dev \
        -v /var/lib/containers:/var/lib/containers \
        -v .:/output \
        {{ image }}:{{ tag }} \
        bootc install to-disk \
            --composefs-backend \
            --via-loopback /output/{{ disk_name }}.img

# Compress and checksum the disk image for distribution.
compress:
    zstd -T0 -v -o {{ disk_name }}.img.zst {{ disk_name }}.img
    sha256sum {{ disk_name }}.img.zst > {{ disk_name }}.img.zst.sha256

# Build, compress, and checksum the disk image (full distributable artifact).
dist: disk compress

# Write a disk image to a USB-attached SSD.
flash image=(disk_name + ".img"):
    scripts/flash.sh {{ image }}

# Remove generated disk images.
clean:
    rm -f {{ disk_name }}*.img {{ disk_name }}*.img.zst {{ disk_name }}*.img.zst.sha256
