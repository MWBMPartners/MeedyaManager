#!/usr/bin/env bash
# ============================================================================
# File: /scripts/generate_icons.sh
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Generates application icon assets from the SVG logo for all platforms.
# Requires: ImageMagick (convert/magick) or rsvg-convert + iconutil (macOS)
#
# Output:
#   assets/icon.png   — 512x512 PNG (Linux, app icon, tray icon)
#   assets/icon.ico   — Multi-resolution Windows icon (16-256px)
#   assets/icon.icns  — macOS icon bundle
#
# Usage:
#   ./scripts/generate_icons.sh
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SVG_SOURCE="${PROJECT_ROOT}/branding/meedyamanager-logo.svg"
ASSETS_DIR="${PROJECT_ROOT}/assets"

# Ensure assets directory exists
mkdir -p "$ASSETS_DIR"

# Check for ImageMagick
if command -v magick &>/dev/null; then
    CONVERT="magick"
elif command -v convert &>/dev/null; then
    CONVERT="convert"
else
    echo "ERROR: ImageMagick (convert or magick) is required."
    echo "  macOS:   brew install imagemagick"
    echo "  Ubuntu:  sudo apt install imagemagick"
    echo "  Windows: choco install imagemagick"
    exit 1
fi

echo "Using ImageMagick: $CONVERT"
echo "SVG source: $SVG_SOURCE"
echo ""

# --- Generate PNG icon (512x512) ---
echo "Generating assets/icon.png (512x512)..."
$CONVERT -background none -density 300 "$SVG_SOURCE" -resize 512x512 "${ASSETS_DIR}/icon.png"
echo "  Done."

# --- Generate Windows ICO (multi-resolution) ---
echo "Generating assets/icon.ico (16/32/48/64/128/256)..."
SIZES=(16 32 48 64 128 256)
ICO_PARTS=()
for size in "${SIZES[@]}"; do
    PART="${ASSETS_DIR}/icon_${size}.png"
    $CONVERT -background none -density 300 "$SVG_SOURCE" -resize "${size}x${size}" "$PART"
    ICO_PARTS+=("$PART")
done
$CONVERT "${ICO_PARTS[@]}" "${ASSETS_DIR}/icon.ico"
# Clean up intermediate PNGs
for part in "${ICO_PARTS[@]}"; do
    rm -f "$part"
done
echo "  Done."

# --- Generate macOS ICNS (if on macOS with iconutil) ---
if [[ "$(uname)" == "Darwin" ]] && command -v iconutil &>/dev/null; then
    echo "Generating assets/icon.icns (macOS icon bundle)..."
    ICONSET_DIR="${ASSETS_DIR}/icon.iconset"
    mkdir -p "$ICONSET_DIR"

    # iconutil requires specific filenames at specific resolutions
    declare -A ICNS_SIZES=(
        ["icon_16x16.png"]=16
        ["icon_16x16@2x.png"]=32
        ["icon_32x32.png"]=32
        ["icon_32x32@2x.png"]=64
        ["icon_128x128.png"]=128
        ["icon_128x128@2x.png"]=256
        ["icon_256x256.png"]=256
        ["icon_256x256@2x.png"]=512
        ["icon_512x512.png"]=512
        ["icon_512x512@2x.png"]=1024
    )

    for name in "${!ICNS_SIZES[@]}"; do
        size="${ICNS_SIZES[$name]}"
        $CONVERT -background none -density 300 "$SVG_SOURCE" \
            -resize "${size}x${size}" "${ICONSET_DIR}/${name}"
    done

    iconutil -c icns "$ICONSET_DIR" -o "${ASSETS_DIR}/icon.icns"

    # Clean up iconset directory
    rm -rf "$ICONSET_DIR"
    echo "  Done."
else
    echo "Skipping macOS ICNS generation (not on macOS or iconutil not found)."
fi

echo ""
echo "Icon generation complete:"
ls -la "${ASSETS_DIR}/icon."*
