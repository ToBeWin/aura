from __future__ import annotations

from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter
import shutil
import subprocess


ROOT = Path("/Users/bingo/workspace/opc/Aura/aura")
ICONS = ROOT / "src-tauri" / "icons"
ICONSET = ICONS / "aura.iconset"
MASTER = ICONS / "aura-mark.png"


def rounded_rect_mask(size: int, radius: float) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    inset = size * 0.06
    draw.rounded_rectangle(
        (inset, inset, size - inset, size - inset),
        radius=radius,
        fill=255,
    )
    return mask


def make_icon(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    mask = rounded_rect_mask(size, size * 0.24)

    bg = Image.new("RGBA", (size, size), (9, 9, 11, 255))
    glow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow)
    glow_draw.ellipse(
        (size * 0.16, size * 0.1, size * 0.88, size * 0.92),
        fill=(44, 44, 48, 110),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(radius=size * 0.08))
    bg = Image.alpha_composite(bg, glow)
    bg.putalpha(mask)
    img.alpha_composite(bg)

    draw = ImageDraw.Draw(img)

    center_x = size / 2
    center_y = size / 2
    bar_width = size * 0.1
    gap = size * 0.065
    bars = [
        (center_x - gap - bar_width, center_y - size * 0.12, center_y + size * 0.12),
        (center_x - bar_width / 2, center_y - size * 0.2, center_y + size * 0.2),
        (center_x + gap, center_y - size * 0.09, center_y + size * 0.09),
    ]
    for left, top, bottom in bars:
        draw.rounded_rectangle(
            (left, top, left + bar_width, bottom),
            radius=bar_width / 2,
            fill=(245, 244, 240, 255),
        )

    dot_r = size * 0.04
    dot_center = (center_x + size * 0.19, center_y - size * 0.18)
    draw.ellipse(
        (
            dot_center[0] - dot_r,
            dot_center[1] - dot_r,
            dot_center[0] + dot_r,
            dot_center[1] + dot_r,
        ),
        fill=(205, 181, 133, 255),
    )

    outline = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    outline_draw = ImageDraw.Draw(outline)
    inset = size * 0.06
    outline_draw.rounded_rectangle(
        (inset, inset, size - inset, size - inset),
        radius=size * 0.24,
        outline=(255, 255, 255, 18),
        width=max(1, int(size * 0.01)),
    )
    img.alpha_composite(outline)
    return img


def save_png(size: int, target: Path) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    make_icon(size).save(target, format="PNG")


def main() -> None:
    if ICONSET.exists():
        shutil.rmtree(ICONSET)
    ICONSET.mkdir(parents=True, exist_ok=True)

    sizes = {
        "icon_16x16.png": 16,
        "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32,
        "icon_32x32@2x.png": 64,
        "icon_128x128.png": 128,
        "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256,
        "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512,
        "icon_512x512@2x.png": 1024,
    }

    for name, size in sizes.items():
        save_png(size, ICONSET / name)

    mappings = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
        "StoreLogo.png": 50,
        "icon.png": 512,
    }

    icon_master = make_icon(1024)
    icon_master.save(MASTER, format="PNG")

    for name, size in mappings.items():
        save_png(size, ICONS / name)


if __name__ == "__main__":
    main()
