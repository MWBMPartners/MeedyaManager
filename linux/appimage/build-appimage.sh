#!/usr/bin/env bash
# (C) 2025-2026 MWBM Partners Ltd
#
# MeedyaManager — AppImage Build Script
#
# Builds an AppImage for Linux x86_64 using appimagetool.
#
# ⚠️  NOT WIRED INTO release.yml (issue #202).
#     Step 3 below does not yet bundle GTK4/libadwaita and their transitive
#     dependencies, so the AppImage this script produces is NOT self-contained:
#     it runs only on hosts whose GTK4 stack is at least as new as the
#     builder's.  Shipping that to testers is worse than shipping nothing, so
#     the release workflow deliberately omits the AppImage and says so in the
#     release body.  Re-add the step in .github/workflows/release.yml once the
#     bundling below is real (linuxdeploy, or an ldd walk into AppDir/usr/lib).
#
# Prerequisites:
#   - Rust toolchain (stable)
#   - GTK4 + libadwaita dev headers: apt-get install -y libgtk-4-dev libadwaita-1-dev gettext
#   - appimagetool: https://github.com/AppImage/AppImageKit/releases
#     wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
#     chmod +x appimagetool-x86_64.AppImage
#
# Usage:
#   bash linux/appimage/build-appimage.sh [VERSION]   # run from the repository root
#
# Output:
#   MeedyaManager-<VERSION>-x86_64.AppImage

set -euo pipefail

VERSION="${1:-$(grep -A20 '\[workspace\.package\]' Cargo.toml | grep '^version' | head -1 | sed 's/.*"\(.*\)"/\1/')}"
APP_NAME="MeedyaManager"
APP_ID="ltd.MWBMpartners.MeedyaManager"
APPDIR="AppDir"
OUTPUT="${APP_NAME}-${VERSION}-x86_64.AppImage"

echo "==> Building MeedyaManager v${VERSION} AppImage"

# ---------------------------------------------------------------------------
# 1. Build the Rust binaries in release mode
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
# 2. Assemble the AppDir skeleton
# ---------------------------------------------------------------------------
echo "==> Assembling AppDir…"
rm -rf "${APPDIR}"

# Binaries — these are the real `[[bin]] name` values.  There is no file
# called mm-gtk or mm-cli; those are crate names.
install -Dm755 target/release/meedya-gtk          "${APPDIR}/usr/bin/meedya-gtk"
install -Dm755 target/release/meedya              "${APPDIR}/usr/bin/meedya"

# No compatibility symlink is needed: the .desktop entry now declares
# `Exec=meedya-gtk`, matching the binary name.  `mm-gtk` is the CRATE name and
# has never been the name of a shipped executable.

# Desktop entry
install -Dm644 linux/flatpak/ltd.MWBMpartners.MeedyaManager.desktop \
    "${APPDIR}/${APP_ID}.desktop"
install -Dm644 linux/flatpak/ltd.MWBMpartners.MeedyaManager.desktop \
    "${APPDIR}/usr/share/applications/${APP_ID}.desktop"

# AppStream metadata
install -Dm644 linux/flatpak/ltd.MWBMpartners.MeedyaManager.metainfo.xml \
    "${APPDIR}/usr/share/metainfo/${APP_ID}.metainfo.xml"

# Icons (for desktop integration and appimagetool metadata)
install -Dm644 linux/flatpak/icons/ltd.MWBMpartners.MeedyaManager.svg \
    "${APPDIR}/usr/share/icons/hicolor/scalable/apps/${APP_ID}.svg"
install -Dm644 linux/flatpak/icons/ltd.MWBMpartners.MeedyaManager-256.png \
    "${APPDIR}/usr/share/icons/hicolor/256x256/apps/${APP_ID}.png"
install -Dm644 linux/flatpak/icons/ltd.MWBMpartners.MeedyaManager-256.png \
    "${APPDIR}/${APP_ID}.png"

# License
install -Dm644 LICENSE "${APPDIR}/usr/share/licenses/${APP_ID}/LICENSE"

# Entry point (appimagetool requires an executable AppRun at the AppDir root)
cat > "${APPDIR}/AppRun" <<'APPRUN'
#!/bin/sh
# AppRun — entry point for the MeedyaManager AppImage
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/meedya-gtk" "$@"
APPRUN
chmod +x "${APPDIR}/AppRun"

# ---------------------------------------------------------------------------
# 3. Bundle GTK4 / libadwaita shared libraries  — NOT IMPLEMENTED
# ---------------------------------------------------------------------------
# Until this uses linuxdeploy or a custom ldd walk to copy libgtk-4.so,
# libadwaita-1.so and their transitive dependencies into AppDir/usr/lib/, the
# AppImage is NOT self-contained.  This is why release.yml does not build it.
echo "==> WARNING: GTK4/libadwaita are NOT bundled — this AppImage is not self-contained."
echo "    It will only run on hosts with a GTK4 stack at least as new as this builder's."
echo "    See https://docs.appimage.org/packaging-guide/manual.html"

# ---------------------------------------------------------------------------
# 4. Build the AppImage
# ---------------------------------------------------------------------------
echo "==> Running appimagetool…"
if command -v appimagetool-x86_64.AppImage &>/dev/null; then
    ARCH=x86_64 ./appimagetool-x86_64.AppImage "${APPDIR}" "${OUTPUT}"
elif command -v appimagetool &>/dev/null; then
    ARCH=x86_64 appimagetool "${APPDIR}" "${OUTPUT}"
else
    echo "::warning:: appimagetool not found — AppDir assembled but AppImage not created."
    echo "Download from: https://github.com/AppImage/AppImageKit/releases"
    exit 0
fi

echo "==> Generated: ${OUTPUT}"
sha256sum "${OUTPUT}" > "${OUTPUT}.sha256"
echo "==> Checksum:  ${OUTPUT}.sha256"
