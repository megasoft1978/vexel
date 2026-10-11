#!/usr/bin/env python3
"""Verify headless light/dark, 1x/2x ruler and guide screenshots."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN_DIR = ROOT / "tests/ui/guides-golden"
BASE_SIZE = (640, 480)


def chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def png_from_rgba(width: int, height: int, rgba: bytes) -> bytes:
    if len(rgba) != width * height * 4:
        raise ValueError("renderer returned an unexpected byte count")
    rows = b"".join(
        b"\0" + rgba[y * width * 4:(y + 1) * width * 4]
        for y in range(height)
    )
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"sRGB", b"\0")
        + chunk(b"IDAT", zlib.compress(rows, 9))
        + chunk(b"IEND", b"")
    )


def render(binary: Path, theme: str, scale: int) -> bytes:
    completed = subprocess.run(
        [str(binary), theme, str(scale)], check=True, capture_output=True, timeout=30,
    )
    return png_from_rgba(BASE_SIZE[0] * scale, BASE_SIZE[1] * scale,
                         completed.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path,
                        default=ROOT / "build/mac-debug/bin/viewer_guides_render")
    parser.add_argument("--update", action="store_true",
                        help="write new screenshots and their manifest")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"missing ruler/guide renderer: {binary}")

    screenshots: dict[str, dict[str, object]] = {}
    for scale in (1, 2):
        width, height = BASE_SIZE[0] * scale, BASE_SIZE[1] * scale
        for theme in ("dark", "light"):
            name = f"rulers-guides-{theme}-{scale}x.png"
            content = render(binary, theme, scale)
            digest = hashlib.sha256(content).hexdigest()
            path = GOLDEN_DIR / name
            if args.update:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            elif not path.is_file() or path.read_bytes() != content:
                raise SystemExit(f"{name} differs from its headless golden")
            screenshots[name] = {
                "sha256": digest,
                "width": width,
                "height": height,
                "theme": theme,
                "backing_scale": scale,
                "renderer": "scalar ruler ticks and CoreText numeric labels",
            }

    manifest = json.dumps({"version": 1, "screenshots": screenshots},
                          sort_keys=True, indent=2) + "\n"
    manifest_path = GOLDEN_DIR / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("ruler/guide golden manifest does not match rendered screenshots")
    print("ruler and guide goldens match: light/dark, 1x/2x")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
