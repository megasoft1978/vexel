#!/usr/bin/env python3
"""Render native shell content views offscreen in light and dark appearances."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
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


def os_profile() -> tuple[int, str]:
    """Select the exact screenshot profile for the running macOS major."""
    try:
        version = subprocess.run(
            ["sw_vers", "-productVersion"], check=True,
            capture_output=True, text=True, timeout=15,
        ).stdout
    except (OSError, subprocess.SubprocessError) as exc:
        raise SystemExit(f"cannot determine native golden macOS profile: {exc}") from exc
    match = re.match(r"^(\d+)\.", version.strip())
    if match is None:
        raise SystemExit(f"unrecognized macOS version for native golden profile: {version.strip()!r}")
    major = int(match.group(1))
    if major not in {15, 27}:
        raise SystemExit(f"no native golden profile is defined for macOS {major}")
    return major, f"macos-{major}"


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

    major, profile = os_profile()
    profile_dir = GOLDEN_DIR / profile

    screenshots: dict[str, dict[str, object]] = {}
    for screen in SCREENS:
        for theme in THEMES:
            name = f"{screen}-{theme}.png"
            resource = icon if screen == "app-icon" else fixture
            content = render(binary, screen, theme, resource)
            width, height = struct.unpack(">II", content[16:24])
            logical_width, logical_height = ((256, 256) if screen == "app-icon" else
                ((600, 650) if screen.startswith("welcome-") else
                 ((560, 380) if screen.startswith("settings-") else
                  ((520, 360) if screen == "help" else (620, 480)))))
            expected_dimensions = (logical_width, logical_height)
            if (width, height) != expected_dimensions:
                raise SystemExit(
                    f"{name} has dimensions {width}x{height}; fixed 1x profile "
                    f"requires {expected_dimensions[0]}x{expected_dimensions[1]}"
                )
            golden_path = profile_dir / name
            if args.update:
                profile_dir.mkdir(parents=True, exist_ok=True)
                golden_path.write_bytes(content)
            elif not golden_path.is_file() or golden_path.read_bytes() != content:
                raise SystemExit(f"{name} differs from its native UI golden")
            screenshots[name] = {
                "sha256": hashlib.sha256(content).hexdigest(),
                "width": width,
                "height": height,
                "theme": theme,
                "screen": screen,
                "renderer": "offscreen AppKit content view",
            }

    manifest = json.dumps({"version": 2, "macos_major": major,
                           "pixel_scale": 1, "screenshots": screenshots},
                          sort_keys=True, indent=2) + "\n"
    manifest_path = profile_dir / "manifest.json"
    if args.update:
        manifest_path.write_text(manifest, encoding="utf-8")
    elif not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest:
        raise SystemExit("native shell golden manifest does not match its images")
    print(f"native shell goldens match: 14 offscreen AppKit renders, macOS {major}, fixed 1x")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
