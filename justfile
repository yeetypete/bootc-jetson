# bootc-jetson developer tasks. The justfile is the single entrypoint for
# building, packaging, and flashing images; CI is a thin wrapper over these
# recipes. The toolchain is docker-only (no podman): docker buildx bake builds
# the image, and `bootc install` runs under docker via --source-imgref.

# Image repository for the built images.
image := "docker.io/yeetypete/bootc-jetson"
# Version for image labels and tag suffix (bake strips a leading "v").
version := "v0.0.0"
# Git commit SHA for image labels.
revision := `git rev-parse HEAD 2>/dev/null || echo ""`

# JetPack release these images target.
jetpack := "7.2"

# Variant to build (override: `just variant=thor dist`). Each name is its build dir.
variant := "orin"
target := "jetson-" + variant
tag := variant + "-jp" + jetpack
disk_name := "bootc-jetson-" + variant
disk_size := "10G"

# List available recipes.
default:
    @just --list

# Build the bootc container image with docker buildx bake. Extra args pass through,
# e.g. `just build --push` or `just build '--set *.cache-to=type=gha'`.
build *args:
    IMAGE={{ image }} VERSION={{ version }} REVISION={{ revision }} \
        docker buildx bake {{ target }} {{ args }}

# Convert the built image into a flashable raw disk image via bootc install to-disk.
# Runs under docker; --source-imgref reads the image from the docker daemon, so no
# podman / containers-storage is needed.
disk:
    truncate -s {{ disk_size }} {{ disk_name }}.img
    docker run \
        --rm --privileged \
        --security-opt label=disable \
        -v /var/run/docker.sock:/var/run/docker.sock \
        -v /dev:/dev \
        -v "$PWD:/output" \
        {{ image }}:{{ tag }} \
        bootc install to-disk \
            --source-imgref docker-daemon:{{ image }}:{{ tag }} \
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
