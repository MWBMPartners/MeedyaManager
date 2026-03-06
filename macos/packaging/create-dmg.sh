#!/usr/bin/env bash
# (C) 2025-2026 MWBM Partners Ltd
#
# MeedyaManager — macOS DMG Creation Script
#
# Creates a distributable .dmg disk image containing the MeedyaManager.app
# bundle.  Requires a macOS host with the Xcode command-line tools installed.
#
# Prerequisites:
#   - macOS 14+ (Sonoma) host
#   - Xcode command-line tools: xcode-select --install
#   - create-dmg (optional, for a prettier DMG): brew install create-dmg
#   - Apple Developer ID certificate in the macOS Keychain (for signing)
#   - APPLE_TEAM_ID and APPLE_DEVELOPER_ID environment variables set
#
# Usage:
#   bash macos/packaging/create-dmg.sh [VERSION]
#
# Environment variables:
#   APPLE_DEVELOPER_ID  — e.g. "Developer ID Application: MWBM Partners Ltd (XXXXXXXXXXX)"
#   APPLE_TEAM_ID       — 10-character Apple Team ID
#   APPLE_ID            — Apple ID email for notarization
#   APPLE_APP_PASSWORD  — App-specific password for notarization
#
# Output:
#   MeedyaManager-<VERSION>-macos-arm64.dmg
#   MeedyaManager-<VERSION>-macos-arm64.dmg.sha256

set -euo pipefail

VERSION="${1:-$(grep -A20 '\[workspace\.package\]' Cargo.toml | grep '^version' | head -1 | sed 's/.*"\(.*\)"/\1/')}"
APP_NAME="MeedyaManager"
APP_BUNDLE="${APP_NAME}.app"
DMG_NAME="${APP_NAME}-${VERSION}-macos-arm64.dmg"

echo "==> Building MeedyaManager v${VERSION} macOS package"

# ---------------------------------------------------------------------------
# 1. Build the Rust FFI library + Swift app in release mode
# ---------------------------------------------------------------------------
echo "==> Building mm-ffi dylib…"
cargo build -p mm-ffi --release

echo "==> Building SwiftUI app…"
cd macos
swift build -c release
cd ..

# ---------------------------------------------------------------------------
# 2. Assemble the .app bundle
# ---------------------------------------------------------------------------
echo "==> Assembling ${APP_BUNDLE}…"
CONTENTS="${APP_BUNDLE}/Contents"
mkdir -p "${CONTENTS}/MacOS" "${CONTENTS}/Resources" "${CONTENTS}/Frameworks"

# Copy the Swift executable
cp "macos/.build/release/MeedyaManager" "${CONTENTS}/MacOS/${APP_NAME}"

# Copy the mm-ffi dylib into Frameworks
cp "target/release/libmm_ffi.dylib" "${CONTENTS}/Frameworks/"

# Fix up the dylib rpath so the app bundle can find it at
# @executable_path/../Frameworks/libmm_ffi.dylib (standard macOS bundle rpath)
install_name_tool -change \
    "@rpath/libmm_ffi.dylib" \
    "@executable_path/../Frameworks/libmm_ffi.dylib" \
    "${CONTENTS}/MacOS/${APP_NAME}" 2>/dev/null || true

# Copy Info.plist
cp "macos/MeedyaManager/Info.plist" "${CONTENTS}/Info.plist"

# Copy GPL-2.0-or-later licence file into bundle Resources.
# GPL-2.0-or-later requires the licence to accompany every distributed binary.
cp "LICENSE" "${CONTENTS}/Resources/LICENSE" 2>/dev/null || \
    echo "::warning::LICENSE file not found at repo root — skipping"

# ---------------------------------------------------------------------------
# 3. Code sign the app bundle
# ---------------------------------------------------------------------------
if [ -n "${APPLE_DEVELOPER_ID:-}" ]; then
    echo "==> Code signing with: ${APPLE_DEVELOPER_ID}…"

    # Sign the embedded dylib first (Apple requires inner bundles and frameworks
    # to be individually signed before the outer bundle is signed with --deep).
    # The --options runtime flag enables Hardened Runtime, required for notarization.
    codesign \
        --force \
        --verify \
        --verbose \
        --sign "${APPLE_DEVELOPER_ID}" \
        --options runtime \
        "${CONTENTS}/Frameworks/libmm_ffi.dylib"

    # Sign the complete app bundle (--deep signs any remaining nested code)
    codesign \
        --deep \
        --force \
        --verify \
        --verbose \
        --sign "${APPLE_DEVELOPER_ID}" \
        --entitlements "macos/MeedyaManager.entitlements" \
        --options runtime \
        "${APP_BUNDLE}"
    codesign --verify --deep --strict "${APP_BUNDLE}"
    echo "==> Code signing successful"
else
    echo "::warning::APPLE_DEVELOPER_ID not set — code signing skipped"
fi

# ---------------------------------------------------------------------------
# 4. Create the DMG
# ---------------------------------------------------------------------------
echo "==> Creating ${DMG_NAME}…"

if command -v create-dmg &>/dev/null; then
    # Use create-dmg for a polished disk image with background and layout
    create-dmg \
        --volname "${APP_NAME} ${VERSION}" \
        --window-pos 200 120 \
        --window-size 660 400 \
        --icon-size 100 \
        --icon "${APP_BUNDLE}" 160 185 \
        --hide-extension "${APP_BUNDLE}" \
        --app-drop-link 500 185 \
        "${DMG_NAME}" \
        "${APP_BUNDLE}"
else
    # Fallback: plain hdiutil DMG
    hdiutil create -volname "${APP_NAME} ${VERSION}" \
        -srcfolder "${APP_BUNDLE}" \
        -ov -format UDZO \
        "${DMG_NAME}"
fi

# ---------------------------------------------------------------------------
# 5. Notarize the DMG (requires Apple credentials in environment)
# ---------------------------------------------------------------------------
if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_APP_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
    echo "==> Submitting ${DMG_NAME} for notarization…"
    xcrun notarytool submit "${DMG_NAME}" \
        --apple-id "${APPLE_ID}" \
        --password "${APPLE_APP_PASSWORD}" \
        --team-id "${APPLE_TEAM_ID}" \
        --wait

    echo "==> Stapling notarization ticket…"
    xcrun stapler staple "${DMG_NAME}"
    xcrun stapler validate "${DMG_NAME}"
    echo "==> Notarization complete"
else
    echo "::warning::Apple credentials not set — notarization skipped"
fi

# ---------------------------------------------------------------------------
# 6. Generate checksum
# ---------------------------------------------------------------------------
shasum -a 256 "${DMG_NAME}" > "${DMG_NAME}.sha256"
echo "==> Generated: ${DMG_NAME}"
echo "==> Checksum:  ${DMG_NAME}.sha256"
