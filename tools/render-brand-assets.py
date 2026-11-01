#!/usr/bin/env python3
"""Render original GenoBeacon icons.

The mark is a radar sweep around an italic Tinos "G", drawn from the
SIL Open Font License face in resources/fonts/tinos/. Nothing here reads
or traces BEACN artwork.
"""

from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
FONT_PATH = ROOT / "resources" / "fonts" / "tinos" / "Tinos-Italic.ttf"
OUT = ROOT / "resources" / "icons"


def g_commands(height: float) -> tuple[str, float, float]:
    font = TTFont(str(FONT_PATH))
    glyph_set = font.getGlyphSet()
    glyph_name = font.getBestCmap()[ord("G")]
    glyph = glyph_set[glyph_name]
    bounds = BoundsPen(glyph_set)
    glyph.draw(bounds)
    xmin, ymin, xmax, ymax = bounds.bounds
    scale = height / (ymax - ymin)
    width = (xmax - xmin) * scale
    pen = SVGPathPen(glyph_set)
    # y-down, ink origin at the top-left of the glyph box
    transform = TransformPen(
        pen, (scale, 0, 0, -scale, -xmin * scale, ymax * scale)
    )
    glyph.draw(transform)
    return pen.getCommands() or "", width, height


def draw_icon(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    margin = size * 0.05
    radius = size * 0.18
    draw.rounded_rectangle(
        [margin, margin, size - margin, size - margin],
        radius=radius,
        fill=(7, 8, 12, 255),
    )
    inset = size * 0.085
    draw.rounded_rectangle(
        [inset, inset, size - inset, size - inset],
        radius=radius * 0.82,
        outline=(225, 6, 0, 255),
        width=max(2, round(size / 42)),
    )

    cx = cy = size / 2
    outer = size * 0.33
    width = max(3, round(size / 26))
    # Open at the bottom so it reads as a dial, not a flame.
    draw.arc(
        [cx - outer, cy - outer, cx + outer, cy + outer],
        start=200,
        end=340,
        fill=(61, 126, 255, 255),
        width=width,
    )
    end = math.radians(340)
    # PIL arc angles are clockwise from 3 o'clock in screen space? Pillow's
    # arc uses degrees clockwise from the 3 o'clock position when y grows down
    # the same way as standard SVG... empirically start=200 end=340 is the
    # upper sweep. Put the beacon dot at the end of that sweep.
    dot_a = math.radians(340 - 90)  # convert pillow-ish; placed explicitly below
    # Place the dot by sampling the same angle Pillow uses: 0° is +x, clockwise.
    dot = math.radians(340)
    dx = cx + math.cos(dot) * outer
    dy = cy + math.sin(dot) * outer
    r = size * 0.045
    draw.ellipse([dx - r, dy - r, dx + r, dy + r], fill=(225, 6, 0, 255))
    del dot_a

    g_h = size * 0.40
    font = ImageFont.truetype(str(FONT_PATH), int(g_h))
    box = draw.textbbox((0, 0), "G", font=font)
    tw, th = box[2] - box[0], box[3] - box[1]
    draw.text(
        (cx - tw / 2 - box[0], cy - th / 2 - box[1] - size * 0.02),
        "G",
        font=font,
        fill=(236, 238, 244, 255),
    )
    return img


def svg_text() -> str:
    size = 1024
    commands, gw, gh = g_commands(size * 0.40)
    cx = cy = size / 2
    x = cx - gw / 2
    y = cy - gh / 2 - size * 0.02
    margin = size * 0.05
    radius = size * 0.18
    inset = size * 0.085
    outer = size * 0.33
    stroke = max(3, round(size / 26))
    # Match Pillow's arc: degrees clockwise from +x.
    a0 = math.radians(200)
    a1 = math.radians(340)
    large = 0
    x0 = cx + math.cos(a0) * outer
    y0 = cy + math.sin(a0) * outer
    x1 = cx + math.cos(a1) * outer
    y1 = cy + math.sin(a1) * outer
    dot = size * 0.045
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">
  <title>GenoBeacon</title>
  <desc>Original GenoBeacon mark: a radar sweep around an italic Tinos G. Not a BEACN logo.</desc>
  <rect x="{margin:.1f}" y="{margin:.1f}" width="{size - 2 * margin:.1f}" height="{size - 2 * margin:.1f}" rx="{radius:.1f}" fill="#07080C"/>
  <rect x="{inset:.1f}" y="{inset:.1f}" width="{size - 2 * inset:.1f}" height="{size - 2 * inset:.1f}" rx="{radius * 0.82:.1f}" fill="none" stroke="#E10600" stroke-width="{max(2, round(size / 42))}"/>
  <path d="M {x0:.1f} {y0:.1f} A {outer:.1f} {outer:.1f} 0 {large} 1 {x1:.1f} {y1:.1f}" fill="none" stroke="#3D7EFF" stroke-width="{stroke}" stroke-linecap="round"/>
  <circle cx="{x1:.1f}" cy="{y1:.1f}" r="{dot:.1f}" fill="#E10600"/>
  <path transform="translate({x:.1f} {y:.1f})" d="{commands}" fill="#ECEEF4"/>
</svg>
"""


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    draw_icon(256).save(OUT / "genobeacon-large.png")
    draw_icon(48).save(OUT / "genobeacon.png")
    (OUT / "genobeacon.svg").write_text(svg_text())
    print("wrote genobeacon icons")


if __name__ == "__main__":
    main()
