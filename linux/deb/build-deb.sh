#!/usr/bin/env bash
# (C) 2025-2026 MWBM Partners Ltd
#
# MeedyaManager — Debian Package Build Script
#
# Builds a .deb package for Debian/Ubuntu (amd64).
#
# Prerequisites:
#   - Rust toolchain (stable)
#   - GTK4 dev headers: apt-get install -y libgtk-4-dev libadwaita-1-dev gettext
#   - dpkg-deb (included in dpkg package on Debian/Ubuntu)
#
# Usage:
#   bash linux/deb/build-deb.sh [VERSION]      # run from the repository root
#
# Output:
#   meedyamanager_<DEBIAN_VERSION>_amd64.deb
#   meedyamanager_<DEBIAN_VERSION>_amd64.deb.sha256

set -euo pipefail

VERSION="${1:-$(grep -A20 '\[workspace\.package\]' Cargo.toml | grep '^version' | head -1 | sed 's/.*"\(.*\)"/\1/')}"

# ---------------------------------------------------------------------------
# Semver → Debian version mapping
# ---------------------------------------------------------------------------
# Debian orders '~' BEFORE the empty string, and every other character AFTER
# it.  So:
#     1.4.0~alpha.1  <  1.4.0      (correct — the alpha precedes the release)
#     1.4.0-alpha.1  >  1.4.0      (wrong  — the alpha would look newer)
# A semver pre-release therefore has its hyphen rewritten to a tilde before it
# reaches the control file or the package filename.  A plain release version
# has no hyphen and is unaffected.
#
# The tilde goes through a variable rather than being written inline: bash 3.2
# (still the /bin/bash on macOS, where developers run this script by hand)
# keeps the backslash in ${VAR//-/\~} and would emit "1.4.0\~alpha.1", while
# bash 5 strips it.  A variable behaves identically on both.
TILDE='~'
DEB_VERSION="${VERSION//-/${TILDE}}"

PKGDIR="meedyamanager_${DEB_VERSION}_amd64"
OUTPUT="${PKGDIR}.deb"

echo "==> Building MeedyaManager v${VERSION} .deb package"
if [ "${DEB_VERSION}" != "${VERSION}" ]; then
    echo "==> Debian version: ${DEB_VERSION} (semver pre-release hyphen mapped to '~')"
fi

# ---------------------------------------------------------------------------
# 1. Build Rust release binaries
# ---------------------------------------------------------------------------
echo "==> Building release binaries…"

# mm-cli produces the `meedya` binary and is a normal workspace member.
cargo build --release -p mm-cli

# mm-gtk is `exclude`d from the root [workspace] (it depends on gettextrs,
# which needs Linux-only system libraries), so `-p mm-gtk` cannot resolve it —
# it must be built through its own manifest.  `--target-dir target` keeps the
# output next to the workspace binaries at target/release/ instead of the
# crate-local crates/mm-gtk/target/.
cargo build --release --manifest-path crates/mm-gtk/Cargo.toml --target-dir target

# ---------------------------------------------------------------------------
# 2. Assemble the package directory tree
# ---------------------------------------------------------------------------
echo "==> Assembling package tree…"
rm -rf "${PKGDIR}"

# DEBIAN control files
install -Dm644 linux/deb/control "${PKGDIR}/DEBIAN/control"
# Update version in control file (in case a different VERSION was passed)
sed -i "s/^Version:.*/Version: ${DEB_VERSION}/" "${PKGDIR}/DEBIAN/control"

# Binaries — these are the real `[[bin]] name` values.  There is no file
# called mm-gtk or mm-cli; those are crate names.
install -Dm755 target/release/meedya-gtk "${PKGDIR}/usr/bin/meedya-gtk"
install -Dm755 target/release/meedya     "${PKGDIR}/usr/bin/meedya"

# No compatibility symlink is needed: the .desktop entry now declares
# `Exec=meedya-gtk`, matching the binary name.  `mm-gtk` is the CRATE name and
# has never been the name of a shipped executable, so there is nothing to stay
# compatible with.

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

# License
install -Dm644 LICENSE "${PKGDIR}/usr/share/doc/meedyamanager/copyright"

# ---------------------------------------------------------------------------
# 3. Build the .deb
# ---------------------------------------------------------------------------
echo "==> Building .deb…"
dpkg-deb --build --root-owner-group "${PKGDIR}" "${OUTPUT}"

echo "==> Generated: ${OUTPUT}"
sha256sum "${OUTPUT}" > "${OUTPUT}.sha256"
echo "==> Checksum:  ${OUTPUT}.sha256"
