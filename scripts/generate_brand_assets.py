#!/usr/bin/env python3
from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Iterable

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]

BRANDING = ROOT / "branding"
ASSETS = ROOT / "assets"
WINDOWS_ASSETS = ROOT / "windows" / "MeedyaManager" / "Assets"
FLATPAK_ICONS = ROOT / "linux" / "flatpak" / "icons"
APPLE_ICONSET = ROOT / "assets" / "apple" / "AppIcon.appiconset"
MACOS_RESOURCES = ROOT / "macos" / "MeedyaManager" / "Resources"

BG_TOP = (32, 74, 122, 240)
BG_BOTTOM = (20, 24, 64, 240)
RING_COLOR = (120, 205, 255, 220)
M_COLOR = (242, 248, 255, 255)
ACCENT_A = (96, 245, 228, 255)
ACCENT_B = (255, 194, 94, 255)


def ensure_dirs() -> None:
    for d in [BRANDING, ASSETS, WINDOWS_ASSETS, FLATPAK_ICONS, APPLE_ICONSET, MACOS_RESOURCES]:
        d.mkdir(parents=True, exist_ok=True)


def rounded_gradient_icon(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    grad = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(grad)
    for y in range(size):
        t = y / (size - 1)
        r = int(BG_TOP[0] * (1 - t) + BG_BOTTOM[0] * t)
        g = int(BG_TOP[1] * (1 - t) + BG_BOTTOM[1] * t)
        b = int(BG_TOP[2] * (1 - t) + BG_BOTTOM[2] * t)
        a = int(BG_TOP[3] * (1 - t) + BG_BOTTOM[3] * t)
        gdraw.line([(0, y), (size, y)], fill=(r, g, b, a))

    radius = int(size * 0.225)
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    img.paste(grad, (0, 0), mask)

    draw = ImageDraw.Draw(img)

    ring_pad = int(size * 0.15)
    ring_w = max(2, int(size * 0.038))
    draw.ellipse(
        [ring_pad, ring_pad, size - ring_pad, size - ring_pad],
        outline=RING_COLOR,
        width=ring_w,
    )

    # Stylized M waveform for "Meedya"
    lw = max(3, int(size * 0.075))
    points = [
        (size * 0.20, size * 0.72),
        (size * 0.35, size * 0.30),
        (size * 0.50, size * 0.62),
        (size * 0.65, size * 0.30),
        (size * 0.80, size * 0.72),
    ]
    draw.line(points, fill=M_COLOR, width=lw, joint="curve")

    # Play button accent for video.
    tri = [
        (size * 0.64, size * 0.45),
        (size * 0.76, size * 0.52),
        (size * 0.64, size * 0.59),
    ]
    draw.polygon(tri, fill=ACCENT_A)

    # Book tab accent for library/books.
    bw = int(size * 0.17)
    bh = int(size * 0.10)
    bx = int(size * 0.16)
    by = int(size * 0.70)
    draw.rounded_rectangle([bx, by, bx + bw, by + bh], radius=max(2, int(size * 0.018)), fill=ACCENT_B)
    crease_x = bx + int(bw * 0.52)
    draw.line([(crease_x, by + 2), (crease_x, by + bh - 2)], fill=(122, 88, 22, 210), width=max(1, size // 160))

    glow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    g2 = ImageDraw.Draw(glow)
    glow_r = int(size * 0.09)
    g2.ellipse(
        [size * 0.60 - glow_r, size * 0.52 - glow_r, size * 0.60 + glow_r, size * 0.52 + glow_r],
        fill=(120, 255, 240, 80),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(max(1, int(size * 0.015))))
    img = Image.alpha_composite(img, glow)

    return img


def save_png(path: Path, size: int) -> None:
    rounded_gradient_icon(size).save(path, format="PNG")


def centered_icon_canvas(width: int, height: int, icon_scale: float = 0.66) -> Image.Image:
    canvas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    icon_size = int(min(width, height) * icon_scale)
    icon = rounded_gradient_icon(icon_size)
    x = (width - icon_size) // 2
    y = (height - icon_size) // 2
    canvas.paste(icon, (x, y), icon)
    return canvas


def write_svg_icon(path: Path, size: int = 1024) -> None:
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}" fill="none">
  <defs>
    <linearGradient id="mm-bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#204A7A" stop-opacity="0.94"/>
      <stop offset="100%" stop-color="#141840" stop-opacity="0.94"/>
    </linearGradient>
    <filter id="mm-glow" x="-30%" y="-30%" width="160%" height="160%">
      <feGaussianBlur stdDeviation="18" result="blur"/>
      <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <rect x="0" y="0" width="{size}" height="{size}" rx="230" fill="url(#mm-bg)"/>
  <circle cx="{size/2}" cy="{size/2}" r="{size*0.35}" stroke="#78CDFF" stroke-opacity="0.85" stroke-width="{size*0.038}"/>
  <path d="M {size*0.20} {size*0.72} L {size*0.35} {size*0.30} L {size*0.50} {size*0.62} L {size*0.65} {size*0.30} L {size*0.80} {size*0.72}" stroke="#F2F8FF" stroke-width="{size*0.075}" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M {size*0.64} {size*0.45} L {size*0.76} {size*0.52} L {size*0.64} {size*0.59} Z" fill="#60F5E4" filter="url(#mm-glow)"/>
  <rect x="{size*0.16}" y="{size*0.70}" width="{size*0.17}" height="{size*0.10}" rx="{size*0.018}" fill="#FFC25E"/>
  <line x1="{size*0.248}" y1="{size*0.706}" x2="{size*0.248}" y2="{size*0.792}" stroke="#7A5816" stroke-opacity="0.82" stroke-width="{max(2, size*0.006)}"/>
</svg>
'''
    path.write_text(svg, encoding="utf-8")


def write_logo_svg(path: Path) -> None:
    svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="1300" height="360" viewBox="0 0 1300 360" fill="none" role="img" aria-label="MeedyaManager logo">
  <defs>
    <linearGradient id="logo-bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#204A7A" stop-opacity="0.94"/>
      <stop offset="100%" stop-color="#141840" stop-opacity="0.94"/>
    </linearGradient>
  </defs>
  <rect x="30" y="30" width="300" height="300" rx="68" fill="url(#logo-bg)"/>
  <circle cx="180" cy="180" r="108" stroke="#78CDFF" stroke-opacity="0.85" stroke-width="12"/>
  <path d="M 90 245 L 135 120 L 180 215 L 225 120 L 270 245" stroke="#F2F8FF" stroke-width="24" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M 222 165 L 256 184 L 222 203 Z" fill="#60F5E4"/>
  <rect x="78" y="220" width="52" height="32" rx="6" fill="#FFC25E"/>
  <line x1="104" y1="224" x2="104" y2="248" stroke="#7A5816" stroke-width="2"/>

  <text x="380" y="170" fill="#E9F1FF" font-size="106" font-family="Avenir Next, Segoe UI, Helvetica, Arial, sans-serif" font-weight="700">Meedya</text>
  <text x="380" y="262" fill="#99BDF9" font-size="96" font-family="Avenir Next, Segoe UI, Helvetica, Arial, sans-serif" font-weight="500">Manager</text>
  <text x="382" y="307" fill="#8DB0D6" font-size="28" font-family="Avenir Next, Segoe UI, Helvetica, Arial, sans-serif" font-weight="500" letter-spacing="1.2">AUDIO  VIDEO  BOOK LIBRARY TAGGING</text>
</svg>
'''
    path.write_text(svg, encoding="utf-8")


def write_wordmark_svg(path: Path) -> None:
    svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="220" viewBox="0 0 1080 220" fill="none" role="img" aria-label="MeedyaManager wordmark">
  <text x="10" y="105" fill="#E9F1FF" font-size="112" font-family="Avenir Next, Segoe UI, Helvetica, Arial, sans-serif" font-weight="700">Meedya</text>
  <text x="10" y="198" fill="#99BDF9" font-size="96" font-family="Avenir Next, Segoe UI, Helvetica, Arial, sans-serif" font-weight="500">Manager</text>
</svg>
'''
    path.write_text(svg, encoding="utf-8")


def write_animated_logo_svg(path: Path) -> None:
    svg = '''<svg xmlns="http://www.w3.org/2000/svg" width="1300" height="360" viewBox="0 0 1300 360" fill="none" role="img" aria-label="MeedyaManager animated logo">
  <defs>
    <linearGradient id="logo-bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#204A7A" stop-opacity="0.94"/>
      <stop offset="100%" stop-color="#141840" stop-opacity="0.94"/>
    </linearGradient>
  </defs>
  <rect x="30" y="30" width="300" height="300" rx="68" fill="url(#logo-bg)"/>
  <circle cx="180" cy="180" r="108" stroke="#78CDFF" stroke-opacity="0.85" stroke-width="12"/>
  <path d="M 90 245 L 135 120 L 180 215 L 225 120 L 270 245" stroke="#F2F8FF" stroke-width="24" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M 222 165 L 256 184 L 222 203 Z" fill="#60F5E4">
    <animate attributeName="opacity" values="0.5;1;0.5" dur="2.4s" repeatCount="indefinite"/>
  </path>
  <rect x="78" y="220" width="52" height="32" rx="6" fill="#FFC25E"/>

  <text x="380" y="170" fill="#E9F1FF" font-size="106" font-family="Avenir Next, Segoe UI, Helvetica, Arial, sans-serif" font-weight="700">Meedya</text>
  <text x="380" y="262" fill="#99BDF9" font-size="96" font-family="Avenir Next, Segoe UI, Helvetica, Arial, sans-serif" font-weight="500">Manager</text>

  <rect x="380" y="286" width="560" height="4" fill="#60F5E4" opacity="0.7">
    <animate attributeName="width" values="0;560;0" dur="2.8s" repeatCount="indefinite"/>
  </rect>
</svg>
'''
    path.write_text(svg, encoding="utf-8")


def save_ico(path: Path) -> None:
    icon = rounded_gradient_icon(1024)
    icon.save(path, format="ICO", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])


def save_icns(path: Path) -> None:
    icon = rounded_gradient_icon(1024)
    icon.save(path, format="ICNS")


def write_windows_assets() -> None:
    centered_icon_canvas(50, 50, 0.92).save(WINDOWS_ASSETS / "StoreLogo.png", format="PNG")
    centered_icon_canvas(44, 44, 0.92).save(WINDOWS_ASSETS / "Square44x44Logo.png", format="PNG")
    centered_icon_canvas(150, 150, 0.90).save(WINDOWS_ASSETS / "Square150x150Logo.png", format="PNG")
    centered_icon_canvas(310, 150, 0.76).save(WINDOWS_ASSETS / "Wide310x150Logo.png", format="PNG")
    centered_icon_canvas(620, 300, 0.72).save(WINDOWS_ASSETS / "SplashScreen.png", format="PNG")


def write_linux_assets() -> None:
    write_svg_icon(FLATPAK_ICONS / "ltd.MWBMpartners.MeedyaManager.svg")
    for size in [16, 24, 32, 48, 64, 96, 128, 256, 512]:
        save_png(FLATPAK_ICONS / f"ltd.MWBMpartners.MeedyaManager-{size}.png", size)


def write_apple_iconset() -> None:
    specs = [
        ("icon-20@2x.png", 40),
        ("icon-20@3x.png", 60),
        ("icon-29@2x.png", 58),
        ("icon-29@3x.png", 87),
        ("icon-40@2x.png", 80),
        ("icon-40@3x.png", 120),
        ("icon-60@2x.png", 120),
        ("icon-60@3x.png", 180),
        ("icon-76.png", 76),
        ("icon-76@2x.png", 152),
        ("icon-83.5@2x.png", 167),
        ("icon-1024.png", 1024),

        # visionOS / universal sizes
        ("icon-64.png", 64),
        ("icon-128.png", 128),
        ("icon-256.png", 256),
        ("icon-512.png", 512),

        # macOS iconset sizes
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]
    for name, size in specs:
        save_png(APPLE_ICONSET / name, size)

    contents = {
        "images": [
            {"size": "20x20", "idiom": "iphone", "filename": "icon-20@2x.png", "scale": "2x"},
            {"size": "20x20", "idiom": "iphone", "filename": "icon-20@3x.png", "scale": "3x"},
            {"size": "29x29", "idiom": "iphone", "filename": "icon-29@2x.png", "scale": "2x"},
            {"size": "29x29", "idiom": "iphone", "filename": "icon-29@3x.png", "scale": "3x"},
            {"size": "40x40", "idiom": "iphone", "filename": "icon-40@2x.png", "scale": "2x"},
            {"size": "40x40", "idiom": "iphone", "filename": "icon-40@3x.png", "scale": "3x"},
            {"size": "60x60", "idiom": "iphone", "filename": "icon-60@2x.png", "scale": "2x"},
            {"size": "60x60", "idiom": "iphone", "filename": "icon-60@3x.png", "scale": "3x"},
            {"size": "20x20", "idiom": "ipad", "filename": "icon-40@2x.png", "scale": "2x"},
            {"size": "29x29", "idiom": "ipad", "filename": "icon-29@2x.png", "scale": "2x"},
            {"size": "40x40", "idiom": "ipad", "filename": "icon-76.png", "scale": "2x"},
            {"size": "76x76", "idiom": "ipad", "filename": "icon-76.png", "scale": "1x"},
            {"size": "76x76", "idiom": "ipad", "filename": "icon-76@2x.png", "scale": "2x"},
            {"size": "83.5x83.5", "idiom": "ipad", "filename": "icon-83.5@2x.png", "scale": "2x"},
            {"size": "1024x1024", "idiom": "ios-marketing", "filename": "icon-1024.png", "scale": "1x"},
            {"size": "1024x1024", "idiom": "mac", "filename": "icon_512x512@2x.png", "scale": "2x"},
            {"size": "16x16", "idiom": "mac", "filename": "icon_16x16.png", "scale": "1x"},
            {"size": "16x16", "idiom": "mac", "filename": "icon_16x16@2x.png", "scale": "2x"},
            {"size": "32x32", "idiom": "mac", "filename": "icon_32x32.png", "scale": "1x"},
            {"size": "32x32", "idiom": "mac", "filename": "icon_32x32@2x.png", "scale": "2x"},
            {"size": "128x128", "idiom": "mac", "filename": "icon_128x128.png", "scale": "1x"},
            {"size": "128x128", "idiom": "mac", "filename": "icon_128x128@2x.png", "scale": "2x"},
            {"size": "256x256", "idiom": "mac", "filename": "icon_256x256.png", "scale": "1x"},
            {"size": "256x256", "idiom": "mac", "filename": "icon_256x256@2x.png", "scale": "2x"},
            {"size": "512x512", "idiom": "mac", "filename": "icon_512x512.png", "scale": "1x"},
            {"size": "512x512", "idiom": "mac", "filename": "icon_512x512@2x.png", "scale": "2x"},
            {"size": "128x128", "idiom": "vision", "filename": "icon-128.png", "scale": "1x"},
            {"size": "256x256", "idiom": "vision", "filename": "icon-256.png", "scale": "1x"},
            {"size": "512x512", "idiom": "vision", "filename": "icon-512.png", "scale": "1x"},
        ],
        "info": {"version": 1, "author": "xcode"},
    }
    (APPLE_ICONSET / "Contents.json").write_text(json.dumps(contents, indent=2) + "\n", encoding="utf-8")


def copy_macos_resources() -> None:
    # Bundle macOS icon resources in the app target.
    source_icns = ASSETS / "icon.icns"
    source_svg = ASSETS / "icon.svg"
    (MACOS_RESOURCES / "AppIcon.icns").write_bytes(source_icns.read_bytes())
    (MACOS_RESOURCES / "AppIcon.svg").write_bytes(source_svg.read_bytes())


def main() -> None:
    ensure_dirs()

    write_svg_icon(ASSETS / "icon.svg")
    save_png(ASSETS / "icon.png", 1024)
    save_ico(ASSETS / "icon.ico")
    save_icns(ASSETS / "icon.icns")

    write_svg_icon(BRANDING / "meedyamanager-logo-mark.svg", size=1024)
    write_wordmark_svg(BRANDING / "meedyamanager-wordmark.svg")
    write_logo_svg(BRANDING / "meedyamanager-logo.svg")
    write_animated_logo_svg(BRANDING / "meedyamanager-logo-animated.svg")

    write_windows_assets()
    write_linux_assets()
    write_apple_iconset()
    copy_macos_resources()


if __name__ == "__main__":
    main()
