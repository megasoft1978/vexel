#!/usr/bin/env python3
"""Build, sign, and package the macOS Vexel application bundle."""

from __future__ import annotations

import argparse
import os
import platform
import plistlib
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path
from typing import Sequence

ROOT = Path(__file__).resolve().parents[1]
BUNDLE_ID = "com.vexel.svgworkbench"
DISPLAY_NAME = "Vexel"
SIGN_IDENTITY_ENV = "VEXEL_SIGN_IDENTITY"
NOTARY_PROFILE_ENV = "VEXEL_NOTARY_PROFILE"
PROJECT_VERSION_RE = re.compile(
    r"^\s*project\s*\(\s*vexel\s+VERSION\s+(\d+\.\d+\.\d+)\s+LANGUAGES\b",
    re.IGNORECASE | re.MULTILINE,
)
ICON_POINTS = (16, 32, 128, 256, 512)
ICON_MASTER_SIZE = 1024
ICNS_REPRESENTATIONS = ((16, b"icp4"), (32, b"icp5"), (64, b"icp6"),
                        (128, b"ic07"), (256, b"ic08"), (512, b"ic09"),
                        (1024, b"ic10"))


class PackagingError(RuntimeError):
    """Raised when a required packaging step fails."""


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("bundle", help="build and sign build/Vexel.app")
    commands.add_parser("dmg", help="build the app and create build/Vexel.dmg")
    return parser.parse_args(argv)


def require_macos_arm64() -> None:
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise PackagingError("Vexel app packaging requires Apple Silicon macOS")


def project_version(root: Path = ROOT) -> str:
    try:
        cmake = (root / "CMakeLists.txt").read_text(encoding="utf-8")
    except OSError as exc:
        raise PackagingError(f"could not read CMake project version: {exc}") from exc
    matches = PROJECT_VERSION_RE.findall(cmake)
    if len(matches) != 1:
        raise PackagingError("CMakeLists.txt must define exactly one three-part Vexel project version")
    return matches[0]


def info_plist(version: str) -> dict[str, object]:
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise PackagingError("bundle version must have exactly three numeric components")
    return {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleDisplayName": DISPLAY_NAME,
        "CFBundleExecutable": "svgworkbench",
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": DISPLAY_NAME,
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "CFBundleIconFile": "Vexel.icns",
        "CFBundleDocumentTypes": [
            {
                "CFBundleTypeExtensions": ["svg"],
                "CFBundleTypeName": "SVG Image",
                "CFBundleTypeRole": "Viewer",
                "LSHandlerRank": "Owner",
                "LSItemContentTypes": ["public.svg-image"],
            }
        ],
        "LSApplicationCategoryType": "public.app-category.graphics-design",
        "LSMinimumSystemVersion": "13.0",
        "NSHighResolutionCapable": True,
    }


def _png_chunk(kind: bytes, data: bytes) -> bytes:
    payload = kind + data
    return struct.pack(">I", len(data)) + payload + struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF)


def _master_icon_rows() -> list[bytes]:
    size = ICON_MASTER_SIZE
    inset = 32
    radius = 170
    polygon = ((235, 250), (378, 250), (512, 590), (646, 250),
               (789, 250), (512, 852))
    rows: list[bytes] = []
    for y in range(size):
        scan_y = y + 0.5
        intersections: list[float] = []
        for index, (x1, y1) in enumerate(polygon):
            x2, y2 = polygon[(index + 1) % len(polygon)]
            if (y1 <= scan_y < y2) or (y2 <= scan_y < y1):
                intersections.append(x1 + (scan_y - y1) * (x2 - x1) / (y2 - y1))
        intersections.sort()
        glyph_intervals = [(int(intersections[i]), int(intersections[i + 1]) + 1)
                           for i in range(0, len(intersections) - 1, 2)]
        row = bytearray([0])
        for x in range(size):
            inside = inset <= x < size - inset and inset <= y < size - inset
            if inside and (x < inset + radius or x >= size - inset - radius) and \
                    (y < inset + radius or y >= size - inset - radius):
                corner_x = inset + radius if x < size // 2 else size - inset - radius - 1
                corner_y = inset + radius if y < size // 2 else size - inset - radius - 1
                inside = ((x - corner_x) ** 2 + (y - corner_y) ** 2) <= radius ** 2
            if not inside:
                row.extend((0, 0, 0, 0))
                continue
            if any(start <= x < end for start, end in glyph_intervals):
                row.extend((246, 247, 250, 255))
            else:
                tint = y * 12 // size
                row.extend((28 + tint, 67 + tint, 112 + tint, 255))
        rows.append(bytes(row))
    return rows


def _icon_png(rows: Sequence[bytes], size: int) -> bytes:
    master = ICON_MASTER_SIZE
    scanlines = bytearray()
    for y in range(size):
        source_y = min(master - 1, y * master // size)
        source = rows[source_y]
        scanlines.append(0)
        for x in range(size):
            source_x = min(master - 1, x * master // size)
            offset = 1 + source_x * 4
            scanlines.extend(source[offset:offset + 4])
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + _png_chunk(b"IHDR", header) +
            _png_chunk(b"IDAT", zlib.compress(bytes(scanlines), level=9)) +
            _png_chunk(b"IEND", b""))


def _write_icon_png(path: Path, rows: Sequence[bytes], size: int) -> None:
    path.write_bytes(_icon_png(rows, size))


def generate_iconset(iconset: Path) -> None:
    iconset.mkdir(parents=True, exist_ok=True)
    rows = _master_icon_rows()
    for points in ICON_POINTS:
        _write_icon_png(iconset / f"icon_{points}x{points}.png", rows, points)
        _write_icon_png(iconset / f"icon_{points}x{points}@2x.png", rows, points * 2)


def generate_icns(path: Path) -> None:
    """Create a PNG-backed ICNS container without external artwork or tools."""
    rows = _master_icon_rows()
    chunks: list[bytes] = []
    for size, chunk_type in ICNS_REPRESENTATIONS:
        image = _icon_png(rows, size)
        chunks.append(chunk_type + struct.pack(">I", len(image) + 8) + image)
    payload = b"".join(chunks)
    path.write_bytes(b"icns" + struct.pack(">I", len(payload) + 8) + payload)


def _run(argv: Sequence[str], *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    print("+ " + " ".join(argv), flush=True)
    result = subprocess.run(list(argv), cwd=ROOT, text=True, capture_output=capture, check=False)
    if result.returncode != 0:
        details = "\n".join(part for part in (result.stdout, result.stderr) if part)
        raise PackagingError(f"command failed ({result.returncode}): {' '.join(argv)}\n{details[-3000:]}")
    if capture:
        details = "\n".join(part for part in (result.stdout, result.stderr) if part)
        if details:
            print(details, end="" if details.endswith("\n") else "\n")
    return result


def _sign_identity(env: dict[str, str] | None = None) -> str:
    source = os.environ if env is None else env
    identity = source.get(SIGN_IDENTITY_ENV, "-").strip()
    if not identity or "\n" in identity or "\r" in identity:
        raise PackagingError(f"{SIGN_IDENTITY_ENV} must be a nonempty signing identity")
    return identity


def build_bundle(root: Path = ROOT, env: dict[str, str] | None = None) -> Path:
    require_macos_arm64()
    version = project_version(root)
    build = root / "build"
    app = build / "Vexel.app"
    binary = build / "mac-release/bin/svgworkbench"
    _run(["cmake", "--preset", "mac-release"])
    _run(["cmake", "--build", "--preset", "mac-release", "--target", "svgworkbench"])
    if not binary.is_file():
        raise PackagingError(f"Release executable was not produced: {binary}")

    if app.is_symlink() or app.is_file():
        app.unlink()
    elif app.exists():
        shutil.rmtree(app)
    macos_dir = app / "Contents/MacOS"
    resources_dir = app / "Contents/Resources"
    macos_dir.mkdir(parents=True)
    resources_dir.mkdir(parents=True)
    executable = macos_dir / "svgworkbench"
    shutil.copy2(binary, executable)
    (app / "Contents/PkgInfo").write_bytes(b"APPL????")
    (app / "Contents/Info.plist").write_bytes(
        plistlib.dumps(info_plist(version), fmt=plistlib.FMT_XML, sort_keys=True)
    )

    identity = _sign_identity(env)
    icon = resources_dir / "Vexel.icns"
    generate_icns(icon)

    _run(["codesign", "--force", "--deep", "--options", "runtime", "--sign", identity, str(app)])
    _run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)], capture=True)
    details = _run(["codesign", "-d", "--verbose=4", str(app)], capture=True)
    signed_details = (details.stdout or "") + (details.stderr or "")
    if not re.search(r"flags=0x[0-9a-f]+\([^)]*runtime", signed_details, re.IGNORECASE):
        raise PackagingError("signed application does not advertise the hardened runtime option")
    print(f"Built {app} (version {version}, identity {identity})")
    return app


def build_dmg(root: Path = ROOT, env: dict[str, str] | None = None) -> Path:
    source = os.environ if env is None else env
    identity = _sign_identity(env)
    profile = source.get(NOTARY_PROFILE_ENV, "").strip()
    if profile and identity == "-":
        raise PackagingError(f"{NOTARY_PROFILE_ENV} requires a Developer ID signing identity")
    app = build_bundle(root, env)
    build = root / "build"
    output = build / "Vexel.dmg"
    try:
        with tempfile.TemporaryDirectory(prefix="vexel-dmg-", dir=build) as temporary:
            staging = Path(temporary) / "Vexel"
            staging.mkdir()
            shutil.copytree(app, staging / "Vexel.app", symlinks=True)
            (staging / "Applications").symlink_to("/Applications")
            _run(["hdiutil", "create", "-volname", "Vexel", "-srcfolder", str(staging),
                  "-ov", "-format", "UDZO", str(output)])
        _run(["hdiutil", "verify", str(output)], capture=True)
        if profile:
            _run(["xcrun", "notarytool", "submit", str(output), "--keychain-profile", profile, "--wait"])
            _run(["xcrun", "stapler", "staple", str(output)])
            _run(["xcrun", "stapler", "validate", str(output)], capture=True)
    except PackagingError:
        if output.exists():
            output.unlink()
        raise
    print(f"Built {output}")
    return output


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "bundle":
            build_bundle()
        else:
            build_dmg()
    except (OSError, PackagingError, subprocess.SubprocessError) as exc:
        print(f"packaging failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
