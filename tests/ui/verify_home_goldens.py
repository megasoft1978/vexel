#!/usr/bin/env python3
"""Verify first-run, loading, and recoverable-error screenshots."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN_DIR = ROOT / "tests/ui/home-golden"
SIZES = ((1280, 800), (2560, 1600))
THEMES = ("dark", "light")
STATES = ("empty", "loading", "error", "document-loading", "document-error")


def png_chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def png_from_rgba(width: int, height: int, rgba: bytes) -> bytes:
    if len(rgba) != width * height * 4:
        raise ValueError("home renderer returned an unexpected RGBA length")
    rows = b"".join(b"\0" + rgba[y * width * 4:(y + 1) * width * 4]
                    for y in range(height))
    return (b"\x89PNG\r\n\x1a\n"
            + png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + png_chunk(b"sRGB", b"\0")
            + png_chunk(b"IDAT", zlib.compress(rows, 9))
            + png_chunk(b"IEND", b""))


def render(binary: Path, theme: str, state: str, width: int, height: int) -> bytes:
    result = subprocess.run([str(binary), theme, state, str(width), str(height)],
        check=True, capture_output=True, timeout=60)
    return png_from_rgba(width, height, result.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path,
        default=ROOT / "build/mac-debug/bin/viewer_home_render")
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"missing welcome renderer: {binary}")
    screenshots: dict[str, dict[str, object]] = {}
    for width, height in SIZES:
        for theme in THEMES:
            for state in STATES:
                name = f"home-{theme}-{width}x{height}-{state}.png"
                content = render(binary, theme, state, width, height)
                path = GOLDEN_DIR / name
                if args.update:
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(content)
                elif not path.is_file() or path.read_bytes() != content:
                    raise SystemExit(f"{name} differs from its home-state golden")
                screenshots[name] = {"sha256": hashlib.sha256(content).hexdigest(),
                    "width": width, "height": height, "theme": theme, "state": state,
                    "renderer": "viewer_home scalar RGBA"}
    manifest = json.dumps({"version": 1, "screenshots": screenshots},
        sort_keys=True, indent=2) + "\n"
    manifest_path = GOLDEN_DIR / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("home-state golden manifest differs")
    print("home-state goldens match: 20 screenshots, dark/light, 1x/2x, five states")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
