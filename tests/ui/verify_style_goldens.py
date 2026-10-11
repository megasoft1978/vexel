#!/usr/bin/env python3
"""Verify deterministic scalar-rendered editor style screenshots."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN_DIR = ROOT / "tests/ui/style-golden"
SIZES = ((1280, 800), (2560, 1600))
THEMES = ("dark", "light")
STATES = ("empty", "selection", "editing-field", "error", "narrow",
          "node-edit", "scale-110", "scale-120")


def png_chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return (struct.pack(">I", len(data)) + body +
            struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF))


def png_from_rgba(width: int, height: int, rgba: bytes) -> bytes:
    expected = width * height * 4
    if len(rgba) != expected:
        raise ValueError(f"renderer returned {len(rgba)} bytes; expected {expected}")
    rows = b"".join(
        b"\0" + rgba[y * width * 4:(y + 1) * width * 4]
        for y in range(height)
    )
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + png_chunk(b"sRGB", b"\0")
        + png_chunk(b"IDAT", zlib.compress(rows, 9))
        + png_chunk(b"IEND", b"")
    )


def layout_width(width: int, state: str) -> int:
    if state != "narrow":
        return width
    # Keep a 450 pt editor viewport at both 1x and 2x pixel density.
    return 450 if width == 1280 else 900


def render(binary: Path, theme: str, width: int, height: int,
           state: str, viewport_width: int) -> bytes:
    completed = subprocess.run(
        [str(binary), theme, str(width), str(height), state, str(viewport_width)],
        check=True, capture_output=True, timeout=60,
    )
    return png_from_rgba(width, height, completed.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path,
                        default=ROOT / "build/mac-debug/bin/viewer_chrome_render")
    parser.add_argument("--update", action="store_true",
                        help="write new screenshots and their manifest")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"missing style renderer: {binary}")

    screenshots: dict[str, dict[str, object]] = {}
    for width, height in SIZES:
        for theme in THEMES:
            for state in STATES:
                viewport_width = layout_width(width, state)
                name = f"editor-{theme}-{width}x{height}-{state}.png"
                content = render(binary, theme, width, height, state, viewport_width)
                digest = hashlib.sha256(content).hexdigest()
                path = GOLDEN_DIR / name
                if args.update:
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(content)
                elif not path.is_file() or path.read_bytes() != content:
                    raise SystemExit(f"{name} differs from its style golden")
                screenshots[name] = {
                    "sha256": digest,
                    "width": width,
                    "height": height,
                    "theme": theme,
                    "state": state,
                    "viewport_width": viewport_width,
                    "renderer": "deterministic scalar text and cached path icons",
                }

    manifest = json.dumps({"version": 1, "screenshots": screenshots},
                          sort_keys=True, indent=2) + "\n"
    manifest_path = GOLDEN_DIR / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("editor style golden manifest does not match rendered screenshots")
    print("editor style goldens match: 32 screenshots, dark/light, 1x/2x, eight states")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
