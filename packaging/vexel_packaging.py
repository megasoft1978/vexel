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
    icon = commands.add_parser("icon", help="generate a PNG-backed ICNS file")
    icon.add_argument("--output", type=Path, required=True)
    icon.add_argument("--master-png", type=Path)
    icon.add_argument("--svg-source", type=Path)
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
                "LSHandlerRank": "Alternate",
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


_ICON_OUTER_CURVES = (
    ((144, 365), (250, 204), (524, 172), (706, 252)),
    ((706, 252), (880, 324), (912, 492), (793, 562)),
    ((793, 562), (692, 626), (610, 540), (523, 464)),
    ((523, 464), (405, 357), (293, 385), (244, 505)),
    ((244, 505), (202, 605), (231, 687), (292, 748)),
    ((292, 748), (107, 690), (62, 520), (144, 365)),
)
_ICON_INNER_CURVES = (
    ((246, 382), (386, 272), (601, 269), (715, 357)),
    ((715, 357), (787, 416), (779, 482), (732, 512)),
    ((732, 512), (687, 540), (646, 489), (588, 444)),
    ((588, 444), (497, 362), (402, 337), (315, 398)),
    ((315, 398), (265, 434), (241, 487), (260, 548)),
    ((260, 548), (211, 489), (210, 423), (246, 382)),
)


def _flatten_icon_form(curves: Sequence[Sequence[tuple[int, int]]]) -> tuple[tuple[float, float], ...]:
    points: list[tuple[float, float]] = []
    subdivisions = 24
    for p0, p1, p2, p3 in curves:
        for step in range(subdivisions):
            t = step / subdivisions
            inverse = 1.0 - t
            x = (inverse ** 3 * p0[0] + 3 * inverse ** 2 * t * p1[0] +
                 3 * inverse * t ** 2 * p2[0] + t ** 3 * p3[0])
            y = (inverse ** 3 * p0[1] + 3 * inverse ** 2 * t * p1[1] +
                 3 * inverse * t ** 2 * p2[1] + t ** 3 * p3[1])
            points.append((x, y))
    return tuple(points)


def _icon_spans(points: Sequence[tuple[float, float]], scan_y: float) -> list[tuple[int, int]]:
    intersections: list[float] = []
    for index, (x1, y1) in enumerate(points):
        x2, y2 = points[(index + 1) % len(points)]
        if (y1 <= scan_y < y2) or (y2 <= scan_y < y1):
            intersections.append(x1 + (scan_y - y1) * (x2 - x1) / (y2 - y1))
    intersections.sort()
    spans = []
    for index in range(0, len(intersections) - 1, 2):
        start = max(0, int(intersections[index] + 0.5))
        end = min(ICON_MASTER_SIZE, int(intersections[index + 1] + 0.5))
        if end > start:
            spans.append((start, end))
    return spans


def _paint_icon_spans(row: bytearray, spans: Sequence[tuple[int, int]],
                      color: tuple[int, int, int, int]) -> None:
    pixel = bytes(color)
    for start, end in spans:
        for x in range(start, end):
            offset = 1 + x * 4
            row[offset:offset + 4] = pixel


def _master_icon_rows() -> list[bytes]:
    """Render the two nested, filled Bézier ribbons at the 1024px source size."""
    outer = _flatten_icon_form(_ICON_OUTER_CURVES)
    inner = _flatten_icon_form(_ICON_INNER_CURVES)
    rows: list[bytes] = []
    for y in range(ICON_MASTER_SIZE):
        row = bytearray([0])
        for x in range(ICON_MASTER_SIZE):
            row.extend((22 + x * 16 // 1024, 26 + x * 38 // 1024 + y * 9 // 1024,
                        63 + x * 67 // 1024 + y * 13 // 1024, 255))
        _paint_icon_spans(row, _icon_spans(outer, y + 0.5), (237, 249, 250, 255))
        _paint_icon_spans(row, _icon_spans(inner, y + 0.5), (70, 226, 204, 255))
        rows.append(bytes(row))
    return rows


def _svg_form_path(curves: Sequence[Sequence[tuple[int, int]]]) -> str:
    first = curves[0][0]
    commands = [f"M{first[0]} {first[1]}"]
    for _start, first_control, second_control, end in curves:
        commands.append("C" + " ".join(
            f"{point[0]} {point[1]}" for point in (first_control, second_control, end)))
    return " ".join(commands) + " Z"


def icon_svg_source() -> str:
    outer = _svg_form_path(_ICON_OUTER_CURVES)
    inner = _svg_form_path(_ICON_INNER_CURVES)
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">\n'
        '  <defs>\n'
        '    <linearGradient id="background" x1="0" y1="0" x2="1" y2="0">\n'
        '      <stop offset="0" stop-color="#161a3f"/>\n'
        '      <stop offset="1" stop-color="#263f82"/>\n'
        '    </linearGradient>\n'
        '    <linearGradient id="background-depth" x1="0" y1="0" x2="0" y2="1">\n'
        '      <stop offset="0" stop-color="#003650" stop-opacity="0"/>\n'
        '      <stop offset="1" stop-color="#003650" stop-opacity="0.16"/>\n'
        '    </linearGradient>\n'
        '  </defs>\n'
        '  <rect width="1024" height="1024" fill="url(#background)"/>\n'
        '  <rect width="1024" height="1024" fill="url(#background-depth)"/>\n'
        f'  <path fill="#edf9fa" d="{outer}"/>\n'
        f'  <path fill="#46e2cc" d="{inner}"/>\n'
        '</svg>\n'
    )


def generate_svg_source(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(icon_svg_source(), encoding="utf-8")


def _icon_png(rows: Sequence[bytes], size: int) -> bytes:
    master = ICON_MASTER_SIZE
    if size <= 0 or size > master or master % size != 0:
        raise PackagingError("icon representation size must evenly divide the 1024px master")
    factor = master // size
    scanlines = bytearray()
    for y in range(size):
        scanlines.append(0)
        for x in range(size):
            totals = [0, 0, 0, 0]
            for source_y in range(y * factor, (y + 1) * factor):
                source = rows[source_y]
                for source_x in range(x * factor, (x + 1) * factor):
                    offset = 1 + source_x * 4
                    for channel in range(4):
                        totals[channel] += source[offset + channel]
            pixels = factor * factor
            scanlines.extend(value // pixels for value in totals)
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


def generate_master_png(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(_icon_png(_master_icon_rows(), ICON_MASTER_SIZE))


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
    built_app = build / "mac-release/Vexel.app"
    _run(["cmake", "--preset", "mac-release"])
    _run(["cmake", "--build", "--preset", "mac-release", "--target", "vexel_app_bundle"])
    if not (built_app / "Contents/MacOS/svgworkbench").is_file():
        raise PackagingError(f"Release app bundle was not produced: {built_app}")
    if not (built_app / "Contents/Resources/Vexel.icns").is_file():
        raise PackagingError(f"Release app icon was not produced: {built_app}")

    if app.is_symlink() or app.is_file():
        app.unlink()
    elif app.exists():
        shutil.rmtree(app)
    shutil.copytree(built_app, app, symlinks=True)
    localization_table = app / "Contents/Resources/en.lproj/Localizable.strings"
    localization_source = root / "resources" / "en.lproj"
    if not localization_source.is_dir():
        localization_source = ROOT / "resources" / "en.lproj"
    if localization_source.is_dir():
        shutil.copytree(localization_source, localization_table.parent,
                        dirs_exist_ok=True)
    if not localization_table.is_file():
        raise PackagingError(f"generated app bundle is missing localization table: {localization_table}")
    try:
        with (app / "Contents/Info.plist").open("rb") as stream:
            manifest = plistlib.load(stream)
    except (OSError, plistlib.InvalidFileException) as exc:
        raise PackagingError(f"generated app Info.plist could not be read: {exc}") from exc
    if manifest.get("CFBundleIdentifier") != BUNDLE_ID or \
            manifest.get("CFBundleVersion") != version:
        raise PackagingError("generated app metadata does not match the project identity/version")

    identity = _sign_identity(env)
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
        if args.command == "icon":
            args.output.parent.mkdir(parents=True, exist_ok=True)
            generate_icns(args.output)
            if args.master_png is not None:
                generate_master_png(args.master_png)
            if args.svg_source is not None:
                generate_svg_source(args.svg_source)
        elif args.command == "bundle":
            build_bundle()
        else:
            build_dmg()
    except (OSError, PackagingError, subprocess.SubprocessError) as exc:
        print(f"packaging failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
