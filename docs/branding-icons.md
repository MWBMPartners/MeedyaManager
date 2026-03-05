# MeedyaManager Logos And Icons

This repository now includes a generated logo/icon set for MeedyaManager ("MediaManager") in vector and raster formats with transparent alpha.

## Source Files

- `scripts/generate_brand_assets.py`: reproducible generator for all branding and app icon assets.
- `assets/icon.svg`: master icon glyph (vector).
- `assets/icon.png`: 1024x1024 transparent PNG.
- `assets/icon.ico`: multi-size Windows icon.
- `assets/icon.icns`: macOS icon container.
- `branding/meedyamanager-logo.svg`: primary horizontal logo.
- `branding/meedyamanager-logo-animated.svg`: animated SVG logo.
- `branding/meedyamanager-logo-mark.svg`: icon-only logo mark.
- `branding/meedyamanager-wordmark.svg`: text-only wordmark.

## Platform Attachments

- Windows (WinUI/MSIX):
  - `windows/MeedyaManager/Assets/StoreLogo.png`
  - `windows/MeedyaManager/Assets/Square44x44Logo.png`
  - `windows/MeedyaManager/Assets/Square150x150Logo.png`
  - `windows/MeedyaManager/Assets/Wide310x150Logo.png`
  - `windows/MeedyaManager/Assets/SplashScreen.png`
  - Referenced by `windows/MeedyaManager/Package.appxmanifest`.

- Linux (Flatpak/Snap/AppImage/Deb):
  - `linux/flatpak/icons/ltd.MWBMpartners.MeedyaManager.svg`
  - `linux/flatpak/icons/ltd.MWBMpartners.MeedyaManager-<size>.png` (16, 24, 32, 48, 64, 96, 128, 256, 512)
  - Installed via:
    - `linux/flatpak/ltd.MWBMpartners.MeedyaManager.yaml`
    - `linux/snap/snapcraft.yaml`
    - `linux/appimage/build-appimage.sh`
    - `linux/deb/build-deb.sh`

- macOS (Swift package app):
  - `macos/MeedyaManager/Resources/AppIcon.icns`
  - `macos/MeedyaManager/Resources/AppIcon.svg`
  - Bundled via `macos/Package.swift` and referenced by `macos/MeedyaManager/Info.plist` (`CFBundleIconFile=AppIcon`).

- Apple mobile/spatial icon set (for iOS, iPadOS, visionOS targets):
  - `assets/apple/AppIcon.appiconset/`
  - Includes `Contents.json` and size variants for iPhone/iPad/mac/vision idioms.
  - Attach this app icon set to any future Xcode iOS/iPadOS/visionOS targets.

## Regeneration

Use either:

```bash
python3 scripts/generate_brand_assets.py
```

or:

```bash
just icons
```
