# hadolint global ignore=DL3008,DL3059

FROM ubuntu:24.04 AS bootc-builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

ARG DESTDIR=/out
ARG RUST_VERSION=1.96.0
ARG BOOTC_REPO=https://github.com/bootc-dev/bootc.git
ARG BOOTC_VERSION=946874cfacef88e596b3e6233feaf4df698d5dd4
ARG OSTREE_VERSION=v2026.1

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y \
    autoconf \
    automake \
    bison \
    build-essential \
    ca-certificates \
    curl \
    git \
    go-md2man \
    gtk-doc-tools \
    libcurl4-openssl-dev \
    libext2fs-dev \
    libfuse3-dev \
    libglib2.0-dev \
    libgpgme-dev \
    liblzma-dev \
    libsoup-3.0-dev \
    libssl-dev \
    libsystemd-dev \
    libtool \
    libzstd-dev \
    meson \
    pkg-config

# bootc requires ostree >= 2025.3 but Ubuntu 24.04 ships 2024.5, so build from source.
WORKDIR /ostree
RUN git clone \
    --depth 1 \
    --branch ${OSTREE_VERSION} \
    --recurse-submodules \
    --shallow-submodules \
    --jobs 8 \
    https://github.com/ostreedev/ostree.git . && \
    env NOCONFIGURE=1 ./autogen.sh && \
    ./configure \
    --prefix=/usr \
    --libdir=/usr/lib \
    --sysconfdir=/etc \
    --with-curl \
    --with-dracut \
    --with-libsystemd && \
    make -j"$(nproc)" && \
    make install DESTDIR=${DESTDIR} && \
    make install DESTDIR=/ && \
    ldconfig

RUN bash -c 'set -euo pipefail; curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain ${RUST_VERSION}'

WORKDIR /bootc
# CARGO_FEATURES=rhel9 selects composefs-rs loopback EROFS mounts, which work on
# the Jetson's 6.8 kernel (file-backed EROFS needs Linux 6.12+).
RUN git init && \
    git remote add origin ${BOOTC_REPO} && \
    git fetch --depth 1 origin ${BOOTC_VERSION} && \
    git checkout FETCH_HEAD && \
    make bin install-all DESTDIR=${DESTDIR} CARGO_FEATURES=rhel9


# Built on bookworm so the binary links against a glibc no newer than Ubuntu 24.04's.
FROM rust:1.98-bookworm AS bootc-imagectl

ARG BOOTC_IMAGECTL_REV=f54932ea30b8a4189fe6099b30eac38f1746be6f
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo install --locked --root /out \
    --git https://github.com/yeetypete/bootc-imagectl --rev "${BOOTC_IMAGECTL_REV}" \
    bootc-imagectl

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

SHELL ["/bin/bash", "-euo", "pipefail", "-c"]

COPY --from=bootc-builder /out/usr/ /usr/
COPY --from=bootc-builder /out/etc/ /etc/
RUN ldconfig

# The apt pin keeps initramfs-tools out, since finalize builds the initramfs with
# dracut. /etc/resolv.conf must be a symlink before the first RUN, because docker
# bind-mounts a regular one and finalize cannot remove a mount.
COPY rootfs/etc/ /etc/

# Create every user and group with the UID and GID the sysusers lock file fixes,
# before any package can allocate a different one.
COPY rootfs/usr/lib/sysusers.d/ /usr/lib/sysusers.d/
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y systemd && \
    systemd-sysusers

# NVIDIA Jetson repos for JetPack 7.2 (Jetson Linux r39.2). 'som' serves the
# Jetson modules (Orin and Thor share this suite), 'common' carries CUDA,
# TensorRT, and friends.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y \
    ca-certificates \
    curl \
    gpg

RUN curl -fsSL "https://repo.download.nvidia.com/jetson/jetson-ota-public.asc" \
    | gpg --dearmor -o /usr/share/keyrings/nvidia-jetson.gpg
COPY <<EOF /etc/apt/sources.list.d/nvidia-jetson.list
deb [signed-by=/usr/share/keyrings/nvidia-jetson.gpg] https://repo.download.nvidia.com/jetson/common r39.2 main
deb [signed-by=/usr/share/keyrings/nvidia-jetson.gpg] https://repo.download.nvidia.com/jetson/som r39.2 main
EOF

# Stub update-initramfs (NVIDIA packages expect it, but we use dracut), and skip
# the L4T preinst QSPI boot-firmware flash, which assumes a live install.
RUN printf '#!/bin/sh\nexit 0\n' > /usr/sbin/update-initramfs && \
    chmod +x /usr/sbin/update-initramfs && \
    mkdir -p /opt/nvidia/l4t-packages && \
    touch /opt/nvidia/l4t-packages/.nv-l4t-disable-boot-fw-update-in-preinstall

ARG KERNEL_RELEASE=6.8.0-1035-nvidia-tegra
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y \
    avahi-daemon \
    bubblewrap \
    dosfstools \
    dracut \
    e2fsprogs \
    efibootmgr \
    fdisk \
    less \
    libnss-mdns \
    libnss-systemd \
    linux-image-${KERNEL_RELEASE} \
    linux-modules-extra-${KERNEL_RELEASE} \
    linux-modules-tegra-igpu-nvgpu-jetson-${KERNEL_RELEASE} \
    linux-modules-tegra-igpu-openrm-jetson-${KERNEL_RELEASE} \
    linux-modules-tegra-oot-jetson-${KERNEL_RELEASE} \
    nvidia-l4t-3d-core \
    nvidia-l4t-bootloader \
    nvidia-l4t-camera \
    nvidia-l4t-core \
    nvidia-l4t-cuda-nvgpu \
    nvidia-l4t-cuda-openrm \
    nvidia-l4t-firmware \
    nvidia-l4t-firmware-nvgpu \
    nvidia-l4t-firmware-openrm \
    nvidia-l4t-gbm \
    nvidia-l4t-gstreamer \
    nvidia-l4t-init \
    nvidia-l4t-multimedia \
    nvidia-l4t-multimedia-utils \
    nvidia-l4t-nvfancontrol \
    nvidia-l4t-nvml \
    nvidia-l4t-nvpmodel \
    nvidia-l4t-nvsci \
    nvidia-l4t-pva \
    nvidia-l4t-tools \
    openssh-server \
    podman \
    skopeo \
    systemd \
    systemd-boot \
    systemd-resolved \
    systemd-timesyncd \
    ubuntu-minimal \
    whois \
    xfsprogs \
    zstd && \
    rm -f /etc/modprobe.d/tegra-igpu-jetson-defaults_nvidia-display_nvidia-tegra.conf

COPY rootfs/ /

# Replace the masked nv-load-gpu-libs.service, which writes to read-only /usr.
# CUDA resolves through /run/nvidia-gpu-libs, which jetson-gpu-select.service
# links to the nvgpu (Orin) or openrm (Thor) libraries at boot. Both sets ship
# the same file names, so the ld.so cache built here against openrm fits either.
RUN ln -sfnT /opt/nvidia/l4t-gpu-libs/openrm /run/nvidia-gpu-libs; \
    ln -sfnT /run/nvidia-gpu-libs/libcuda.so /usr/lib/aarch64-linux-gnu/libcuda.so; \
    ln -sfnT /run/nvidia-gpu-libs/libcuda.so.1 /usr/lib/aarch64-linux-gnu/libcuda.so.1; \
    echo /run/nvidia-gpu-libs > /etc/ld.so.conf.d/000-nvidia-gpu-libs.conf; \
    ldconfig

# Ubuntu ships tmp.mount in /usr/share/systemd/ (opt-in). Copy it to the unit
# search path so systemctl enable can find it.
RUN cp /usr/share/systemd/tmp.mount /usr/lib/systemd/system/

RUN systemctl enable \
    avahi-daemon.service \
    jetson-gpu-select.service \
    nvidia-display-load.service \
    ssh.service \
    systemd-growfs@sysroot.service \
    systemd-networkd.service \
    systemd-resolved.service \
    systemd-timesyncd.service \
    tmp.mount && \
    systemctl disable \
    apt-daily-upgrade.service \
    apt-daily-upgrade.timer \
    apt-daily.service \
    apt-daily.timer && \
    # nv-l4t-bootloader-config.service is not bootc-aware (runs lsblk on the
    # composefs root). It is safe to mask, since UEFI and nvbootctrl read board
    # identity from EEPROM directly. nvpmodel.service stays enabled and a drop-in
    # limits it to Orin.
    rm -f \
    /etc/systemd/system/multi-user.target.wants/nv-l4t-bootloader-config.service \
    /etc/systemd/system/multi-user.target.wants/nv-load-display-modules.service \
    /etc/systemd/system/sysinit.target.wants/nv-load-gpu-libs.service \
    /etc/systemd/system/nv-graphics.service \
    /etc/systemd/system/nv-l4t-bootloader-config.service \
    /etc/systemd/system/nv-load-display-modules.service \
    /etc/systemd/system/nv-load-gpu-libs.service && \
    systemctl mask \
    nv-graphics.service \
    nv-l4t-bootloader-config.service \
    nv-load-display-modules.service \
    nv-load-gpu-libs.service

# Create home directory on first login and configure the ubuntu user. Drop the
# pam_lastlog.so line, since the module is gone from Ubuntu 24.04 and fails to dlopen.
RUN pam-auth-update --enable mkhomedir && \
    sed -i '/pam_lastlog\.so/d' /etc/pam.d/login && \
    usermod -p "$(mkpasswd -m sha-512 'ubuntu')" ubuntu && \
    echo "ubuntu ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/ubuntu && \
    chmod 440 /etc/sudoers.d/ubuntu

LABEL containers.bootc=1

# Lay out the filesystem, fix the accounts, stage the kernel, build the initramfs,
# relocate the dpkg database under /usr, empty /var into tmpfiles.d entries and
# lint the result.
COPY --from=bootc-imagectl /out/bin/bootc-imagectl /usr/libexec/bootc-imagectl
RUN /usr/libexec/bootc-imagectl finalize \
    --sysusers-lock /usr/lib/sysusers.d/00-bootc-imagectl.lock.conf
