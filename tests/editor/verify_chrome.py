#!/usr/bin/env python3
"""Render and verify deterministic 1280x800 editor chrome goldens."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN_DIR = ROOT / "tests/editor/chrome-golden"
WIDTH = 1280
HEIGHT = 800


def profile_dir() -> tuple[int, Path]:
    try:
        version = subprocess.run(["sw_vers", "-productVersion"], check=True,
                                 capture_output=True, text=True, timeout=15).stdout.strip()
    except (OSError, subprocess.SubprocessError) as exc:
        raise SystemExit(f"cannot determine editor chrome macOS profile: {exc}") from exc
    match = re.match(r"^(\d+)\.", version)
    if match is None or int(match.group(1)) not in {15, 27}:
        raise SystemExit(f"no editor chrome profile is defined for macOS {version!r}")
    major = int(match.group(1))
    return major, GOLDEN_DIR / f"macos-{major}"


def chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def png_from_rgba(rgba: bytes) -> bytes:
    if len(rgba) != WIDTH * HEIGHT * 4:
        raise ValueError("editor renderer returned an unexpected byte count")
    rows = b"".join(b"\0" + rgba[y * WIDTH * 4:(y + 1) * WIDTH * 4] for y in range(HEIGHT))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 6, 0, 0, 0))
            + chunk(b"sRGB", b"\0") + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b""))


def render(binary: Path, theme: str) -> bytes:
    completed = subprocess.run([str(binary), theme], check=True, capture_output=True, timeout=30)
    return png_from_rgba(completed.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "build/mac-debug/bin/editor_chrome_render")
    parser.add_argument("--update", action="store_true", help="write current screenshots and manifest")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"missing editor renderer: {binary}")
    major, profile = profile_dir()
    entries = {}
    for theme in ("dark", "light"):
        name = f"editor-chrome-{theme}.png"
        content = render(binary, theme)
        digest = hashlib.sha256(content).hexdigest()
        path = profile / name
        if args.update:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        elif not path.is_file() or path.read_bytes() != content:
            raise SystemExit(f"{theme} editor chrome differs from its golden")
        entries[name] = {"sha256": digest, "width": WIDTH, "height": HEIGHT}
    manifest = json.dumps({"version": 2, "macos_major": major,
                           "screenshots": entries}, sort_keys=True, indent=2) + "\n"
    manifest_path = profile / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("editor chrome golden manifest does not match rendered screenshots")
    print(f"editor chrome goldens match: macOS {major}, dark/light (1280x800)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
