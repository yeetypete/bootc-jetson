# bootc-jetson developer tasks.

# Image repository for the built images.
image := "yeetypete/bootc-jetson"
# Version for image labels and tag suffix (docker bake strips a leading "v").
version := "v0.0.0"
# Git commit SHA for image labels.
revision := `git rev-parse HEAD 2>/dev/null || echo ""`
# JetPack release these images target.
jetpack := "7.2"
# Variant to build, e.g. `just variant=orin dist`. Each name is the variant's build dir.
variant := "orin"
# Whether `build` also pushes images to the registry (set push=true on releases).
push := "false"

target := "jetson-" + variant
tag := variant + "-jp" + jetpack
disk_name := "bootc-jetson-" + variant
disk_size := "10G"
oci_archive := "image.oci"

# List available recipes.
default:
    @just --list

# Build the bootc container image.
build *args:
    IMAGE={{ image }} VERSION={{ version }} REVISION={{ revision }} PUSH={{ push }} \
        docker buildx bake {{ target }} {{ args }}

# Convert the built image into a flashable raw disk image via bootc install to-disk.
disk:
    truncate -s {{ disk_size }} {{ disk_name }}.img
    docker run \
        --rm --privileged \
        --security-opt label=disable \
        -v /dev:/dev \
        -v "$PWD:/output" \
        {{ image }}:{{ tag }} \
        bootc install to-disk \
            --source-imgref oci-archive:/output/{{ oci_archive }}:{{ tag }} \
            --target-imgref docker.io/{{ image }}:{{ tag }} \
            --composefs-backend \
            --via-loopback /output/{{ disk_name }}.img

# Compress and checksum the disk image for distribution.
compress:
    zstd -T0 -v -o {{ disk_name }}.img.zst {{ disk_name }}.img
    sha256sum {{ disk_name }}.img.zst > {{ disk_name }}.img.zst.sha256

# Build, compress, and checksum the disk image.
dist: disk compress

# Write a disk image to an external storage device.
flash image=(disk_name + ".img"):
    scripts/flash.sh {{ image }}

# Remove generated disk images and the OCI archive.
clean:
    rm -f {{ disk_name }}*.img {{ disk_name }}*.img.zst {{ disk_name }}*.img.zst.sha256 {{ oci_archive }}
