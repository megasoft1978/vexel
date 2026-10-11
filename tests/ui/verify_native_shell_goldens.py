#!/usr/bin/env python3
"""Render native shell content views offscreen in light and dark appearances."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN_DIR = ROOT / "tests/ui/native-shell-golden"
SCREENS = (
    "welcome-empty",
    "welcome-recent",
    "settings-general",
    "settings-canvas",
    "help",
    "feedback",
    "app-icon",
)
THEMES = ("light", "dark")


def render(binary: Path, screen: str, theme: str, recent_fixture: Path) -> bytes:
    command = [str(binary), screen, theme]
    if screen in {"welcome-recent", "app-icon"}:
        command.append(str(recent_fixture))
    completed = subprocess.run(command, check=True, capture_output=True, timeout=30)
    if not completed.stdout.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError(f"renderer returned invalid PNG data for {screen}-{theme}")
    return completed.stdout


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--recent-fixture", type=Path, required=True)
    parser.add_argument("--icon-png", type=Path, required=True)
    parser.add_argument("--update", action="store_true",
                        help="write rendered images and their manifest")
    args = parser.parse_args()
    binary = args.binary.resolve()
    # Keep relative fixtures relative so the golden shows a stable parent path
    # across worktrees instead of embedding a developer's checkout location.
    fixture = args.recent_fixture
    icon = args.icon_png.resolve()
    if not binary.is_file() or not fixture.is_file() or not icon.is_file():
        raise SystemExit("missing native renderer, recent SVG fixture, or app icon PNG")

    screenshots: dict[str, dict[str, object]] = {}
    for screen in SCREENS:
        for theme in THEMES:
            name = f"{screen}-{theme}.png"
            resource = icon if screen == "app-icon" else fixture
            content = render(binary, screen, theme, resource)
            if args.update:
                GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
                (GOLDEN_DIR / name).write_bytes(content)
            elif not (GOLDEN_DIR / name).is_file() or \
                    (GOLDEN_DIR / name).read_bytes() != content:
                raise SystemExit(f"{name} differs from its native UI golden")
            width, height = struct.unpack(">II", content[16:24])
            screenshots[name] = {
                "sha256": hashlib.sha256(content).hexdigest(),
                "width": width,
                "height": height,
                "theme": theme,
                "screen": screen,
                "renderer": "offscreen AppKit content view",
            }

    manifest = json.dumps({"version": 1, "screenshots": screenshots},
                          sort_keys=True, indent=2) + "\n"
    manifest_path = GOLDEN_DIR / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("native shell golden manifest does not match its images")
    print("native shell goldens match: 14 offscreen AppKit renders, light/dark")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
