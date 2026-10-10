#!/usr/bin/env python3
"""Verify deterministic selection overlay PNGs for both themes and scales."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN_DIR = ROOT / "tests/editor/selection-golden"
POINTS = (1280, 800)


def chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return (struct.pack(">I", len(data)) + body +
            struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF))


def png_from_rgba(rgba: bytes, width: int, height: int) -> bytes:
    if len(rgba) != width * height * 4:
        raise ValueError("selection renderer returned an unexpected byte count")
    rows = b"".join(b"\0" + rgba[y * width * 4:(y + 1) * width * 4]
                    for y in range(height))
    return (b"\x89PNG\r\n\x1a\n" +
            chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) +
            chunk(b"sRGB", b"\0") + chunk(b"IDAT", zlib.compress(rows, 9)) +
            chunk(b"IEND", b""))


def render(binary: Path, theme: str, scale: int) -> bytes:
    completed = subprocess.run([str(binary), theme, str(scale)], check=True,
                               capture_output=True, timeout=60)
    return png_from_rgba(completed.stdout, POINTS[0] * scale,
                         POINTS[1] * scale)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path,
                        default=ROOT / "build/mac-debug/bin/editor_selection_render")
    parser.add_argument("--update", action="store_true",
                        help="write new selection screenshots and manifest")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"missing selection renderer: {binary}")
    entries = {}
    for theme in ("dark", "light"):
        for scale in (1, 2):
            name = f"selection-{theme}-{scale}x.png"
            content = render(binary, theme, scale)
            digest = hashlib.sha256(content).hexdigest()
            path = GOLDEN_DIR / name
            if args.update:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            elif not path.is_file() or path.read_bytes() != content:
                raise SystemExit(f"{name} differs from its selection golden")
            entries[name] = {"sha256": digest, "width": POINTS[0] * scale,
                             "height": POINTS[1] * scale}
    manifest = json.dumps({"version": 1, "screenshots": entries},
                          sort_keys=True, indent=2) + "\n"
    path = GOLDEN_DIR / "manifest.json"
    if args.update:
        path.write_text(manifest, encoding="utf-8")
    elif not path.is_file() or path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("selection golden manifest does not match rendered screenshots")
    print("editor selection goldens match: dark/light at 1x/2x")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
