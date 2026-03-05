#!/usr/bin/env bash
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# MeedyaManager — AppImage Build Script
#
# Builds a self-contained AppImage for Linux x86_64 using appimagetool.
#
# Prerequisites:
#   - Rust toolchain (stable)
#   - GTK4 + libadwaita dev headers: apt-get install -y libgtk-4-dev libadwaita-1-dev
#   - appimagetool: https://github.com/AppImage/AppImageKit/releases
#     wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
#     chmod +x appimagetool-x86_64.AppImage
#
# Usage:
#   bash linux/appimage/build-appimage.sh [VERSION]
#
# Output:
#   MeedyaManager-<VERSION>-x86_64.AppImage

set -euo pipefail

VERSION="${1:-$(grep -A20 '\[workspace\.package\]' Cargo.toml | grep '^version' | head -1 | sed 's/.*"\(.*\)"/\1/')}"
APP_NAME="MeedyaManager"
APP_ID="com.mwbm.MeedyaManager"
APPDIR="AppDir"
OUTPUT="${APP_NAME}-${VERSION}-x86_64.AppImage"

echo "==> Building MeedyaManager v${VERSION} AppImage"

# ---------------------------------------------------------------------------
# 1. Build the Rust binaries in release mode
# ---------------------------------------------------------------------------
echo "==> Building release binaries…"
cargo build --release -p mm-gtk -p mm-cli

# ---------------------------------------------------------------------------
# 2. Assemble the AppDir skeleton
# ---------------------------------------------------------------------------
echo "==> Assembling AppDir…"
rm -rf "${APPDIR}"

# Binary
install -Dm755 target/release/mm-gtk             "${APPDIR}/usr/bin/mm-gtk"
install -Dm755 target/release/mm-cli             "${APPDIR}/usr/bin/mm-cli"

# Desktop entry
install -Dm644 linux/flatpak/com.mwbm.MeedyaManager.desktop \
    "${APPDIR}/${APP_ID}.desktop"
install -Dm644 linux/flatpak/com.mwbm.MeedyaManager.desktop \
    "${APPDIR}/usr/share/applications/${APP_ID}.desktop"

# AppStream metadata
install -Dm644 linux/flatpak/com.mwbm.MeedyaManager.metainfo.xml \
    "${APPDIR}/usr/share/metainfo/${APP_ID}.metainfo.xml"

# Symlink for appimagetool (requires AppRun or a symlink to the binary)
cat > "${APPDIR}/AppRun" <<'APPRUN'
#!/bin/sh
# AppRun — entry point for the MeedyaManager AppImage
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/mm-gtk" "$@"
APPRUN
chmod +x "${APPDIR}/AppRun"

# ---------------------------------------------------------------------------
# 3. Bundle GTK4 / libadwaita shared libraries
# ---------------------------------------------------------------------------
# In production, use linuxdeployqt or a custom ldd-based bundler to copy
# libgtk-4.so, libadwaita-1.so, and their transitive dependencies into
# AppDir/usr/lib/ so the AppImage is self-contained.
echo "==> Note: GTK4/libadwaita bundling requires linuxdeploy or manual ldd walk."
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
