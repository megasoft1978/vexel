#!/usr/bin/env python3
"""Verify dark/light headless chrome images at 200% interface text size."""

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
GOLDEN_DIR = ROOT / "tests/editor/accessibility-localization-golden"
WIDTH, HEIGHT = 1280, 800
INTERFACE_SCALE = 200


def profile_dir() -> tuple[int, Path]:
    try:
        version = subprocess.run(["sw_vers", "-productVersion"], check=True,
                                 capture_output=True, text=True, timeout=15).stdout.strip()
    except (OSError, subprocess.SubprocessError) as exc:
        raise SystemExit(f"cannot determine accessibility golden macOS profile: {exc}") from exc
    match = re.match(r"^(\d+)\.", version)
    if match is None or int(match.group(1)) not in {15, 27}:
        raise SystemExit(f"no accessibility golden profile is defined for macOS {version!r}")
    major = int(match.group(1))
    return major, GOLDEN_DIR / f"macos-{major}"


def png_chunk(kind: bytes, data: bytes) -> bytes:
    payload = kind + data
    return (struct.pack(">I", len(data)) + payload +
            struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF))


def png_from_rgba(rgba: bytes) -> bytes:
    if len(rgba) != WIDTH * HEIGHT * 4:
        raise ValueError("editor renderer returned an unexpected byte count")
    rows = b"".join(b"\0" + rgba[row * WIDTH * 4:(row + 1) * WIDTH * 4]
                    for row in range(HEIGHT))
    return (b"\x89PNG\r\n\x1a\n" +
            png_chunk(b"IHDR", struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 6, 0, 0, 0)) +
            png_chunk(b"sRGB", b"\0") +
            png_chunk(b"IDAT", zlib.compress(rows, 9)) + png_chunk(b"IEND", b""))


def render(binary: Path, theme: str) -> bytes:
    result = subprocess.run([str(binary), theme, str(INTERFACE_SCALE)],
                            check=True, capture_output=True, timeout=60)
    return png_from_rgba(result.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path,
                        default=ROOT / "build/mac-debug/bin/editor_chrome_render")
    parser.add_argument("--update", action="store_true",
                        help="write the light/dark accessibility chrome goldens")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"missing headless editor renderer: {binary}")
    major, profile = profile_dir()

    entries: dict[str, dict[str, int | str]] = {}
    for theme in ("dark", "light"):
        name = f"accessibility-chrome-{theme}-{INTERFACE_SCALE}.png"
        content = render(binary, theme)
        digest = hashlib.sha256(content).hexdigest()
        path = profile / name
        if args.update:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        elif not path.is_file() or path.read_bytes() != content:
            raise SystemExit(f"{name} differs from its headless golden")
        entries[name] = {"sha256": digest, "width": WIDTH, "height": HEIGHT}

    manifest = json.dumps({"version": 2, "macos_major": major,
                           "screenshots": entries},
                          sort_keys=True, indent=2) + "\n"
    manifest_path = profile / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("accessibility chrome golden manifest differs")
    print(f"accessibility chrome goldens match: macOS {major}, light/dark at 200% interface text")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
