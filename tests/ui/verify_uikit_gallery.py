#!/usr/bin/env python3
"""Generate or verify deterministic UI kit component and icon sheets."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDENS = ROOT / "tests/ui/uikit-golden"
DOCS = ROOT / "docs/ui"
THEMES = ("light", "dark")
SCALES = (1, 2)
SHEETS = {"components": (1284, 1392), "icons": (462, 534)}


def png_chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def png_from_rgba(width: int, height: int, rgba: bytes) -> bytes:
    expected = width * height * 4
    if len(rgba) != expected:
        raise ValueError(f"renderer returned {len(rgba)} bytes; expected {expected}")
    compressor = zlib.compressobj(level=9)
    compressed = bytearray()
    stride = width * 4
    for y in range(height):
        compressed.extend(compressor.compress(b"\0" + rgba[y * stride:(y + 1) * stride]))
    compressed.extend(compressor.flush())
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + png_chunk(b"sRGB", b"\0")
        + png_chunk(b"IDAT", bytes(compressed))
        + png_chunk(b"IEND", b"")
    )


def render(binary: Path, theme: str, scale: int, width: int, height: int) -> bytes:
    completed = subprocess.run(
        [str(binary), theme, str(scale)], check=True, capture_output=True, timeout=120
    )
    return png_from_rgba(width * scale, height * scale, completed.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--icon-binary", type=Path, required=True)
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    binary, icon_binary = args.binary.resolve(), args.icon_binary.resolve()
    if not binary.is_file() or not icon_binary.is_file():
        raise SystemExit("missing UI gallery executable")

    manifest: dict[str, dict[str, object]] = {}
    for theme in THEMES:
        for scale in SCALES:
            for kind, renderer in (("components", binary), ("icons", icon_binary)):
                width, height = SHEETS[kind]
                image = render(renderer, theme, scale, width, height)
                name = f"{kind}-{theme}-{scale}x.png"
                target = GOLDENS / name
                digest = hashlib.sha256(image).hexdigest()
                if args.update:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(image)
                elif not target.is_file() or target.read_bytes() != image:
                    raise SystemExit(f"{name} differs from its UI kit golden")
                manifest[name] = {
                    "sha256": digest, "width": width * scale, "height": height * scale,
                    "theme": theme, "scale": scale, "sheet": kind,
                }
                if args.update and scale == 2:
                    doc_name = f"uikit-{kind}-gallery-{theme}.png"
                    (DOCS / doc_name).write_bytes(image)

    manifest_path = GOLDENS / "manifest.json"
    content = json.dumps({"version": 1, "images": manifest}, sort_keys=True, indent=2) + "\n"
    if args.update:
        manifest_path.write_text(content, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != content:
        raise SystemExit("UI kit golden manifest does not match its images")
    print("UI kit goldens match: all components and 45 icons, light/dark, 1x/2x")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
