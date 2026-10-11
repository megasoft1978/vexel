#!/usr/bin/env python3
"""Render and verify command-palette and undo-history overlays in both themes."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN_DIR = ROOT / "tests/ui/command-overlay-golden"
WIDTH = 1280
HEIGHT = 800


def png_chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def png_from_rgba(rgba: bytes) -> bytes:
    if len(rgba) != WIDTH * HEIGHT * 4:
        raise ValueError("command overlay renderer returned an unexpected byte count")
    rows = b"".join(
        b"\0" + rgba[y * WIDTH * 4 : (y + 1) * WIDTH * 4]
        for y in range(HEIGHT)
    )
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 6, 0, 0, 0))
        + png_chunk(b"sRGB", b"\0")
        + png_chunk(b"IDAT", zlib.compress(rows, 9))
        + png_chunk(b"IEND", b"")
    )


def render(binary: Path, theme: str, overlay: str) -> bytes:
    completed = subprocess.run(
        [str(binary), theme, overlay],
        cwd=ROOT,
        check=True,
        capture_output=True,
        timeout=30,
    )
    return png_from_rgba(completed.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--binary",
        type=Path,
        default=ROOT / "build/mac-debug/bin/editor_command_overlay_render",
    )
    parser.add_argument("--output-dir", type=Path, help="also write rendered PNGs to this directory")
    parser.add_argument("--update", action="store_true", help="write current screenshots and manifest")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"missing command overlay renderer: {binary}")
    entries = {}
    for overlay in ("palette", "history"):
        for theme in ("dark", "light"):
            name = f"{overlay}-{theme}.png"
            content = render(binary, theme, overlay)
            digest = hashlib.sha256(content).hexdigest()
            path = GOLDEN_DIR / name
            if args.output_dir is not None:
                args.output_dir.mkdir(parents=True, exist_ok=True)
                (args.output_dir / name).write_bytes(content)
            if args.update:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            elif not path.is_file() or path.read_bytes() != content:
                raise SystemExit(f"{theme} {overlay} overlay differs from its golden")
            entries[name] = {"sha256": digest, "width": WIDTH, "height": HEIGHT}
    manifest = json.dumps({"version": 1, "screenshots": entries}, sort_keys=True, indent=2) + "\n"
    manifest_path = GOLDEN_DIR / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("command overlay golden manifest does not match rendered screenshots")
    print("command overlay goldens match: palette/history, dark/light (1280x800)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
