#!/usr/bin/env bash
# (C) 2025-2026 MWBM Partners Ltd
#
# MeedyaManager — Debian Package Build Script
#
# Builds a .deb package for Debian/Ubuntu (amd64).
#
# Prerequisites:
#   - Rust toolchain (stable)
#   - GTK4 dev headers: apt-get install -y libgtk-4-dev libadwaita-1-dev
#   - dpkg-deb (included in dpkg package on Debian/Ubuntu)
#
# Usage:
#   bash linux/deb/build-deb.sh [VERSION]
#
# Output:
#   meedyamanager_<VERSION>_amd64.deb

set -euo pipefail

VERSION="${1:-$(grep -A20 '\[workspace\.package\]' Cargo.toml | grep '^version' | head -1 | sed 's/.*"\(.*\)"/\1/')}"
PKGDIR="meedyamanager_${VERSION}_amd64"
OUTPUT="${PKGDIR}.deb"

echo "==> Building MeedyaManager v${VERSION} .deb package"

# ---------------------------------------------------------------------------
# 1. Build Rust release binaries
# ---------------------------------------------------------------------------
echo "==> Building release binaries…"
cargo build --release -p mm-gtk -p mm-cli

# ---------------------------------------------------------------------------
# 2. Assemble the package directory tree
# ---------------------------------------------------------------------------
echo "==> Assembling package tree…"
rm -rf "${PKGDIR}"

# DEBIAN control files
install -Dm644 linux/deb/control "${PKGDIR}/DEBIAN/control"
# Update version in control file (in case a different VERSION was passed)
sed -i "s/^Version:.*/Version: ${VERSION}/" "${PKGDIR}/DEBIAN/control"

# Binaries
install -Dm755 target/release/mm-gtk "${PKGDIR}/usr/bin/mm-gtk"
install -Dm755 target/release/mm-cli "${PKGDIR}/usr/bin/mm-cli"

# Desktop integration
install -Dm644 linux/flatpak/ltd.MWBMpartners.MeedyaManager.desktop \
    "${PKGDIR}/usr/share/applications/ltd.MWBMpartners.MeedyaManager.desktop"
install -Dm644 linux/flatpak/ltd.MWBMpartners.MeedyaManager.metainfo.xml \
    "${PKGDIR}/usr/share/metainfo/ltd.MWBMpartners.MeedyaManager.metainfo.xml"
install -Dm644 linux/flatpak/icons/ltd.MWBMpartners.MeedyaManager.svg \
    "${PKGDIR}/usr/share/icons/hicolor/scalable/apps/ltd.MWBMpartners.MeedyaManager.svg"
install -Dm644 linux/flatpak/icons/ltd.MWBMpartners.MeedyaManager-256.png \
    "${PKGDIR}/usr/share/icons/hicolor/256x256/apps/ltd.MWBMpartners.MeedyaManager.png"

# Man page placeholder
install -Dm644 /dev/null "${PKGDIR}/usr/share/man/man1/meedyamanager.1"

# ---------------------------------------------------------------------------
# 3. Build the .deb
# ---------------------------------------------------------------------------
echo "==> Building .deb…"
dpkg-deb --build --root-owner-group "${PKGDIR}" "${OUTPUT}"

echo "==> Generated: ${OUTPUT}"
sha256sum "${OUTPUT}" > "${OUTPUT}.sha256"
echo "==> Checksum:  ${OUTPUT}.sha256"
