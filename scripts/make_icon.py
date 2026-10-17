#!/usr/bin/env python3
"""Draw the Vexel app icon and build `resources/AppIcon.icns`.

Dev-only tool, run by hand; Pillow is NOT a product dependency. The artwork is
first-party: a rounded square (corner radius 22.37% of the shape, 10% transparent
margin on each side) with a cobalt-to-ink gradient and a white Bezier "V" whose
square anchors and circular handles follow vector-editor conventions, plus one
accent keyframe diamond on the path.

Each size is drawn at 4x and downscaled with Lanczos. Small sizes (<= 64 px)
use a simplified variant (no handles, heavier stroke) so the mark stays legible.

Usage: python3 scripts/make_icon.py [--iconset-dir DIR] [--icns FILE] [--no-icns]
Defaults: build/icon/AppIcon.iconset and resources/AppIcon.icns.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
CANVAS = 1024
SUPERSAMPLE = 4
MARGIN = 0.10  # transparent margin per side, fraction of canvas
CORNER = 0.2237  # corner radius as a fraction of the shape size

# Palette from docs/ui/design-direction-2026-10.md: dark accent #5EA3FA, ink
# surfaces #171A1D / #1E2125; cobalt start tone chosen for the icon gradient.
COBALT = (38, 92, 226)
INK = (14, 20, 36)
WHITE = (255, 255, 255)
ACCENT = (94, 163, 250)  # design-direction dark accent

# (point size, filename inside the .iconset)
ICONSET = [
    (16, "icon_16x16.png"),
    (32, "icon_16x16@2x.png"),
    (32, "icon_32x32.png"),
    (64, "icon_32x32@2x.png"),
    (128, "icon_128x128.png"),
    (256, "icon_128x128@2x.png"),
    (256, "icon_256x256.png"),
    (512, "icon_256x256@2x.png"),
    (512, "icon_512x512.png"),
    (1024, "icon_512x512@2x.png"),
]


def _gradient(size: int) -> Image.Image:
    """Diagonal top-left to bottom-right gradient, cobalt to ink."""
    ramp = Image.linear_gradient("L").resize((size, size), Image.BILINEAR)
    horizontal = ramp.transpose(Image.Transpose.TRANSPOSE)
    mask = ImageChops.add(ramp, horizontal, scale=2.0)
    start = Image.new("RGB", (size, size), COBALT)
    end = Image.new("RGB", (size, size), INK)
    return Image.composite(end, start, mask)


def _cubic(p0, p1, p2, p3, steps: int = 64):
    pts = []
    for i in range(steps + 1):
        t = i / steps
        u = 1 - t
        x = u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0]
        y = u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1]
        pts.append((x, y))
    return pts


def _point(p0, p1, p2, p3, t: float):
    u = 1 - t
    return (
        u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
        u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1],
    )


def _draw_mark(canvas: Image.Image, scale: float, detailed: bool) -> None:
    """Draws the white V, anchors, handles and accent keyframe on `canvas`.

    Geometry is authored on the 1024 grid and multiplied by `scale`.
    """
    d = ImageDraw.Draw(canvas)

    def s(v):
        return v * scale

    a = (300, 312)  # top-left anchor
    b = (512, 724)  # bottom anchor
    c = (724, 312)  # top-right anchor
    a_out = (352, 530)  # handle of A
    b_in = (432, 672)  # handle in to B
    b_out = (592, 672)  # handle out of B
    c_in = (672, 530)  # handle of C

    left = _cubic(a, a_out, b_in, b)
    right = _cubic(b, b_out, c_in, c)
    stroke = round(s(52 if detailed else 84))
    path = [(s(x), s(y)) for x, y in left + right[1:]]
    # A row of round dots gives a clean round-capped, round-joined stroke.
    r = stroke / 2
    for x, y in path:
        d.ellipse((x - r, y - r, x + r, y + r), fill=WHITE)

    if detailed:
        handle_colour = (*ACCENT, 255)
        line_w = max(2, round(s(6)))
        for anchor, handle in ((a, a_out), (b, b_in), (b, b_out), (c, c_in)):
            d.line([(s(anchor[0]), s(anchor[1])), (s(handle[0]), s(handle[1]))],
                   fill=handle_colour, width=line_w)
            hr = s(14)
            hx, hy = s(handle[0]), s(handle[1])
            d.ellipse((hx - hr, hy - hr, hx + hr, hy + hr), fill=WHITE,
                      outline=handle_colour, width=line_w)
        for anchor in (a, b, c):
            q = s(26)
            ax, ay = s(anchor[0]), s(anchor[1])
            d.rectangle((ax - q, ay - q, ax + q, ay + q), fill=WHITE,
                        outline=COBALT, width=max(2, round(s(6))))

    # Accent keyframe diamond on the left stroke, halfway along the curve.
    kx, ky = _point(a, a_out, b_in, b, 0.5)
    kr = s(30 if detailed else 50)
    diamond = [(s(kx), s(ky) - kr), (s(kx) + kr, s(ky)), (s(kx), s(ky) + kr), (s(kx) - kr, s(ky))]
    # Cobalt knockout ring separates the accent diamond from the white stroke.
    gap = s(10 if detailed else 14)
    halo = [(s(kx), s(ky) - kr - gap), (s(kx) + kr + gap, s(ky)),
            (s(kx), s(ky) + kr + gap), (s(kx) - kr - gap, s(ky))]
    d.polygon(halo, fill=COBALT)
    d.polygon(diamond, fill=ACCENT)


def render(point_size: int) -> Image.Image:
    """Renders the icon at `point_size` px, drawn at SUPERSAMPLE x and reduced."""
    big = point_size * SUPERSAMPLE
    scale = big / CANVAS
    shape = big * (1 - 2 * MARGIN)
    x0 = y0 = big * MARGIN
    box = (x0, y0, x0 + shape, y0 + shape)
    mask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, radius=shape * CORNER, fill=255)
    # Gradient spans the shape; sized to the shape then placed at its origin.
    body = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    grad = _gradient(round(shape)).convert("RGBA")
    body.paste(grad, (round(x0), round(y0)))
    body.putalpha(mask)
    mark = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    _draw_mark(mark, scale, detailed=point_size > 64)
    body.alpha_composite(mark)
    return body.resize((point_size, point_size), Image.LANCZOS)


def build_iconset(iconset: Path) -> None:
    if iconset.exists():
        shutil.rmtree(iconset)
    iconset.mkdir(parents=True)
    cache: dict[int, Image.Image] = {}
    for size, name in ICONSET:
        if size not in cache:
            cache[size] = render(size)
        cache[size].save(iconset / name, "PNG")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--iconset-dir", type=Path,
                        default=ROOT / "build/icon/AppIcon.iconset")
    parser.add_argument("--icns", type=Path, default=ROOT / "resources/AppIcon.icns")
    parser.add_argument("--no-icns", action="store_true",
                        help="write the .iconset only")
    args = parser.parse_args()
    build_iconset(args.iconset_dir)
    print(f"iconset: {args.iconset_dir}")
    if args.no_icns:
        return 0
    args.icns.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["iconutil", "-c", "icns", str(args.iconset_dir), "-o", str(args.icns)],
                   check=True)
    print(f"icns: {args.icns}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
