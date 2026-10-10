#!/usr/bin/env python3
"""Report required local build, test, and app-packaging tools."""

from __future__ import annotations

import argparse
import platform
import shutil
import subprocess
import sys
from dataclasses import dataclass


@dataclass(frozen=True)
class Tool:
    name: str
    executable: str
    version_args: tuple[str, ...]
    purpose: str


TOOLS = (
    Tool("cmake", "cmake", ("--version",), "configure and build"),
    Tool("ninja", "ninja", ("--version",), "CMake build generator"),
    Tool("clang", "clang", ("--version",), "C17 compiler"),
    Tool("rsvg-convert", "rsvg-convert", ("--version",), "reference-renderer checks"),
    Tool("python3", "python3", ("--version",), "test and packaging scripts"),
    Tool("codesign", "codesign", ("--version",), "application signing verification"),
    Tool("hdiutil", "hdiutil", ("help",), "compressed DMG creation"),
)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args(argv)
    return argparse.Namespace()


def _version(path: str, args: tuple[str, ...]) -> str:
    try:
        result = subprocess.run([path, *args], check=False, capture_output=True, text=True, timeout=5)
    except (OSError, subprocess.SubprocessError):
        return "version unavailable"
    lines = (result.stdout or result.stderr).strip().splitlines()
    if result.returncode != 0 or not lines:
        return "version unavailable"
    return lines[0][:160]


def run_doctor() -> tuple[int, list[str]]:
    lines: list[str] = []
    failed = False
    if sys.platform != "darwin":
        lines.append("FAIL platform: Vexel requires macOS; this checkout does not target other operating systems")
        failed = True
    else:
        lines.append(f"OK platform: macOS {platform.mac_ver()[0] or 'version unavailable'} ({platform.machine()})")
        try:
            major = int(platform.mac_ver()[0].split(".", maxsplit=1)[0])
        except (ValueError, IndexError):
            major = 0
        if major < 13 or platform.machine() != "arm64":
            lines.append("FAIL platform: Apple Silicon macOS 13 or later is required")
            failed = True

    for tool in TOOLS:
        path = shutil.which(tool.executable)
        if path is None:
            lines.append(f"MISSING {tool.name}: required for {tool.purpose}; executable not found on PATH")
            failed = True
        else:
            version = _version(path, tool.version_args)
            lines.append(f"OK {tool.name}: {path} ({version})")
    return (1 if failed else 0), lines


def main(argv: list[str] | None = None) -> int:
    parse_args(argv)
    status, lines = run_doctor()
    for line in lines:
        print(line)
    if status == 0:
        print("All checked prerequisites are available.")
    else:
        print("Resolve the listed prerequisites before building or packaging.", file=sys.stderr)
    return status


if __name__ == "__main__":
    raise SystemExit(main())
