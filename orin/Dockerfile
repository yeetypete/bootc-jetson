# hadolint global ignore=DL3008,DL3059

FROM ubuntu:24.04 AS bootc-builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

ARG DESTDIR=/out
ARG RUST_VERSION=1.96.0
# Pinned to a fork commit carrying the composefs registry-auth fix (restores
# #2086, dropped by the PullOptions migration). Without it `bootc switch/upgrade`
# from a private registry fails with "access denied" on the composefs backend.
# Repoint at bootc-dev/bootc once the fix lands upstream.
ARG BOOTC_REPO=https://github.com/yeetypete/bootc.git
ARG BOOTC_VERSION=f4e612f526bc297501e391be551be33046e843b3
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
# CARGO_FEATURES=rhel9 selects composefs-rs loopback EROFS mounts (plus the
# pre-6.15 string-lowerdir overlay path), which work on the Jetson's 6.8 kernel.
# Without it composefs-rs uses file-backed EROFS, needing CONFIG_EROFS_FS_BACKED_BY_FILE
# (Linux 6.12+). `make bin` builds with --bins, unifying the feature onto the
# bootc-initramfs-setup binary that mounts the composefs image.
RUN git init && \
    git remote add origin ${BOOTC_REPO} && \
    git fetch --depth 1 origin ${BOOTC_VERSION} && \
    git checkout FETCH_HEAD && \
    make bin install-all DESTDIR=${DESTDIR} CARGO_FEATURES=rhel9


# Custom kernel with CONFIG_EROFS_FS + CONFIG_FS_VERITY, which nvidia-l4t-kernel
# ships disabled. bootc's composefs+verity backend needs both at boot (EROFS to
# mount the image, FS_VERITY to measure the sealed objects). It is configured as
# NVIDIA does (`make defconfig` plus our options). OOT modules are rebuilt against
# it below.
FROM ubuntu:24.04 AS kernel-builder

ENV DEBIAN_FRONTEND=noninteractive

# r39.2 kernel source (the Noble 6.8 tree shipped as nvidia-l4t-kernel).
# KERNEL_RELEASE is the stock uname -r, reproduced via LOCALVERSION so the
# modules dir matches the apt-installed nvidia-l4t-kernel.
ARG KERNEL_SRC_REPO=https://gitlab.com/nvidia/nv-tegra/3rdparty/canonical/linux-noble.git
ARG KERNEL_SRC_TAG=jetson_39.2_GA
ARG KERNEL_RELEASE=6.8.12-1021-tegra

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y \
    bc \
    bison \
    build-essential \
    ca-certificates \
    cpio \
    curl \
    flex \
    git \
    kmod \
    libelf-dev \
    libssl-dev \
    openssl \
    python3 \
    rsync \
    zstd

WORKDIR /kernel
RUN git clone --depth 1 --branch "${KERNEL_SRC_TAG}" "${KERNEL_SRC_REPO}" .

# NVIDIA's in-tree defconfig, then EROFS + FS_VERITY built in (=y) so both are
# available in the initramfs without extra dracut wiring.
RUN set -eux; \
    make ARCH=arm64 defconfig; \
    ./scripts/config --file .config \
      --enable FS_VERITY \
      --enable FS_VERITY_BUILTIN_SIGNATURES \
      --enable EROFS_FS \
      --enable EROFS_FS_ZIP \
      --enable EROFS_FS_ZIP_DEFLATE; \
    make ARCH=arm64 olddefconfig

# Native arm64 build. LOCALVERSION=-1021-tegra makes uname -r match the
# apt-installed nvidia-l4t-kernel (and its modules dir). Without it the raw git
# tree builds as "6.8.12+".
RUN set -eux; \
    kver="$(make -s ARCH=arm64 kernelversion)"; \
    suffix="${KERNEL_RELEASE#"$kver"}"; \
    make ARCH=arm64 LOCALVERSION="$suffix" -j"$(nproc)" Image modules; \
    make ARCH=arm64 LOCALVERSION="$suffix" INSTALL_MOD_PATH=/out modules_install; \
    rm -f "/out/lib/modules/${KERNEL_RELEASE}/build" \
          "/out/lib/modules/${KERNEL_RELEASE}/source"; \
    install -D arch/arm64/boot/Image /out/boot/Image

# Rebuild NVIDIA's out-of-tree modules against our kernel (the prebuilt ones are
# built for a non-verity kernel and will not load). Per the r39.2 OOT modules
# guide: fetch sources, point KERNEL_HEADERS at our built tree, build and install.
# The display/gpu trees extract as nvdisplay/ and unifiedgpudisp/, which the OOT
# Makefile expects.
# https://docs.nvidia.com/jetson/archives/r39.2/DeveloperGuide/SD/Kernel/KernelCustomization.html
ARG L4T_PUBLIC_SOURCES=https://developer.nvidia.com/downloads/embedded/l4t/r39_release_v2.0/sources/public_sources.tbz2
RUN --mount=type=cache,target=/dl,sharing=locked \
    set -eux; \
    f=/dl/public_sources.tbz2; \
    [ -f "$f" ] || curl -fsSL -o "$f" "${L4T_PUBLIC_SOURCES}"; \
    mkdir -p /oot-src; \
    tar xjf "$f" --strip-components=2 -C /oot-src \
      Linux_for_Tegra/source/kernel_oot_modules_src.tbz2 \
      Linux_for_Tegra/source/nvidia_kernel_display_driver_source.tbz2 \
      Linux_for_Tegra/source/nvidia_unified_gpu_display_driver_source.tbz2; \
    for t in kernel_oot_modules_src nvidia_kernel_display_driver_source \
             nvidia_unified_gpu_display_driver_source; do \
      tar xjf "/oot-src/${t}.tbz2" -C /oot-src; \
      rm -f "/oot-src/${t}.tbz2"; \
    done

# INSTALL_MOD_DIR=updates matches the stock layout. kernel_name=noble and empty
# CROSS_COMPILE per the guide.
WORKDIR /oot-src
RUN set -eux; \
    export KERNEL_HEADERS=/kernel CROSS_COMPILE=; \
    make -j"$(nproc)" kernel_name=noble system_type=l4t modules; \
    make kernel_name=noble system_type=l4t \
      INSTALL_MOD_PATH=/out INSTALL_MOD_DIR=updates modules_install; \
    rm -f "/out/lib/modules/${KERNEL_RELEASE}/build" \
          "/out/lib/modules/${KERNEL_RELEASE}/source"

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

SHELL ["/bin/bash", "-euo", "pipefail", "-c"]

COPY --from=bootc-builder /out/usr/ /usr/
COPY --from=bootc-builder /out/etc/ /etc/
RUN ldconfig

# bootc requires dracut, so keep initramfs-tools out.
COPY <<EOF /etc/apt/preferences.d/no-initramfs-tools
Package: initramfs-tools*
Pin: release *
Pin-Priority: -1
EOF

# NVIDIA Jetson repos for JetPack 7.2 (Jetson Linux r39.2). 'som' is SoM-generic
# and serves the whole Orin family. 'common' carries CUDA, TensorRT, and friends.
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

# Stub update-initramfs (NVIDIA packages expect it, but we use dracut). Skip the
# L4T preinst boot-firmware flash, which assumes a live install and flashes QSPI
# immediately.
RUN printf '#!/bin/sh\nexit 0\n' > /usr/sbin/update-initramfs && \
    chmod +x /usr/sbin/update-initramfs && \
    mkdir -p /opt/nvidia/l4t-packages && \
    touch /opt/nvidia/l4t-packages/.nv-l4t-disable-boot-fw-update-in-preinstall

# Install the system. nvidia-l4t-kernel and the OOT module packages come in as
# dependencies but we replace their kernel with our rebuild, so strip the stock
# Image + modules in this same layer.
ARG KERNEL_RELEASE=6.8.12-1021-tegra
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install --no-install-recommends -y \
    bubblewrap \
    dosfstools \
    dracut \
    e2fsprogs \
    efibootmgr \
    fdisk \
    less \
    nvidia-l4t-3d-core \
    nvidia-l4t-bootloader \
    nvidia-l4t-camera \
    nvidia-l4t-core \
    nvidia-l4t-cuda-nvgpu \
    nvidia-l4t-display-kernel \
    nvidia-l4t-firmware \
    nvidia-l4t-firmware-nvgpu \
    nvidia-l4t-gbm \
    nvidia-l4t-gstreamer \
    nvidia-l4t-init \
    nvidia-l4t-kernel \
    nvidia-l4t-kernel-dtbs \
    nvidia-l4t-kernel-oot-modules \
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
    rm -rf "/usr/lib/modules/${KERNEL_RELEASE}/kernel" \
           "/usr/lib/modules/${KERNEL_RELEASE}/updates" \
           "/usr/lib/modules/${KERNEL_RELEASE}/extra" \
           /boot/Image

# Install our kernel. Configured from NVIDIA's defconfig with the full OOT source
# rebuilt, so our module set covers the stock one stripped above. depmod re-runs
# in the staging step below.
COPY --from=kernel-builder /out/boot/Image /boot/Image
COPY --from=kernel-builder /out/lib/modules/${KERNEL_RELEASE}/ /usr/lib/modules/${KERNEL_RELEASE}/

COPY rootfs/ /

# Bake the NVIDIA GPU/display driver setup at build time. The nvidia-l4t-init
# load services (nv-load-display-modules, nv-load-gpu-libs) do this at boot, but
# they write to read-only /usr (depmod, libcuda symlink) and break on bootc, so
# replicate their work here and mask them below. Orin is tegra234 -> nvgpu variant.
ARG GPU_VARIANT=nvgpu-l4t
ARG GPU_LIB=nvgpu
RUN cfgdir=/opt/nvidia/nv-disp-module-configs; \
    libdir="/opt/nvidia/l4t-gpu-libs/${GPU_LIB}"; \
    cp "${cfgdir}/nv-depmod-${GPU_VARIANT}-display.conf" /etc/depmod.d/nvidia-display.conf; \
    cp "${cfgdir}/nv-modprobe-${GPU_VARIANT}-display.conf" /etc/modprobe.d/nvidia-display.conf; \
    install -d /usr/lib/aarch64-linux-gnu /etc/ld.so.conf.d; \
    ln -sfnT "$libdir/libcuda.so" /usr/lib/aarch64-linux-gnu/libcuda.so; \
    ln -sfnT "$libdir/libcuda.so.1" /usr/lib/aarch64-linux-gnu/libcuda.so.1; \
    echo "$libdir" > /etc/ld.so.conf.d/000-nvidia-gpu-libs.conf

# Generate the initramfs and stage vmlinuz next to the modules for ostree/bootc.
RUN depmod "${KERNEL_RELEASE}"; \
    ldconfig; \
    dracut --force --no-hostonly --reproducible --zstd --verbose \
      --kver "${KERNEL_RELEASE}" "/usr/lib/modules/${KERNEL_RELEASE}/initramfs.img"; \
    cp /boot/Image "/usr/lib/modules/${KERNEL_RELEASE}/vmlinuz"

# Make the filesystem layout ostree-compatible. Remove the placeholder fstab too.
# ostree's /etc overlay otherwise makes libmount emit a spurious "fstab has been
# modified" warning at runtime.
# hadolint ignore=SC2114
RUN rm -rf /boot /srv /home /root /usr/local /mnt && \
    mkdir -p /boot /sysroot /usr/lib/ostree /var && \
    ln -s /var/home /home && \
    ln -s /var/roothome /root && \
    ln -s /var/srv /srv && \
    ln -s /var/usrlocal /usr/local && \
    ln -s /var/mnt /mnt && \
    ln -s sysroot/ostree /ostree && \
    rm -f /etc/fstab

# Ubuntu ships tmp.mount in /usr/share/systemd/ (opt-in). Copy it to the unit
# search path so systemctl enable can find it.
RUN cp /usr/share/systemd/tmp.mount /usr/lib/systemd/system/

RUN systemctl enable \
    ostree-remount.service \
    ssh.service \
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
    # composefs root and needs /etc/nv_boot_control.conf). On Orin (t23x) its
    # only real job is keeping COMPATIBLE_SPEC current. UEFI and nvbootctrl read
    # board identity from EEPROM directly, so masking it is safe.
    rm -f \
    /etc/systemd/system/nv-load-display-modules.service \
    /etc/systemd/system/nv-load-gpu-libs.service \
    /etc/systemd/system/nv-graphics.service \
    /etc/systemd/system/nv-l4t-bootloader-config.service \
    /etc/systemd/system/multi-user.target.wants/nv-l4t-bootloader-config.service && \
    systemctl mask \
    nv-load-display-modules.service \
    nv-load-gpu-libs.service \
    nv-graphics.service \
    nv-l4t-bootloader-config.service

# Create home directory on first login and configure the ubuntu user. Drop the
# deprecated pam_lastlog.so line (the module was removed from Ubuntu 24.04's
# libpam-modules, so every console login logs a dlopen failure).
RUN pam-auth-update --enable mkhomedir && \
    sed -i '/pam_lastlog\.so/d' /etc/pam.d/login && \
    usermod -p "$(mkpasswd -m sha-512 'ubuntu')" ubuntu && \
    echo "ubuntu ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/ubuntu && \
    chmod 440 /etc/sudoers.d/ubuntu

# Relocate the dpkg database to /usr so it persists across image updates. Content
# in /var is only applied at initial provisioning, so a tmpfiles.d symlink restores
# /var/lib/dpkg at boot. This mirrors rpm's /usr/lib/sysimage/rpm.
RUN mkdir -p /usr/lib/sysimage && \
    mv /var/lib/dpkg /usr/lib/sysimage/dpkg

# Embed registry pull secret for bootc upgrades from a private registry. -s skips
# cleanly when the secret is absent or empty. Local builds proceed without it and
# CI with a real token logs in.
RUN --mount=type=secret,id=ostree-auth,required=false \
    if [ -s /run/secrets/ostree-auth ]; then \
    skopeo login --authfile /usr/lib/ostree/auth.json \
    -u yeetypete --password-stdin docker.io < /run/secrets/ostree-auth; \
    fi

# bootc expects /var empty (populated at boot via tmpfiles.d) and /run, /tmp clean.
RUN rm -rf /run/* /tmp/* /var/log/* && \
    find /var -mindepth 1 -type f -delete && \
    find /var -mindepth 1 -type l -delete && \
    find /var -mindepth 1 -type d -empty -delete

LABEL containers.bootc=1

RUN bootc container lint --fatal-warnings
