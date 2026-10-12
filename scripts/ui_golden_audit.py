#!/usr/bin/env python3
"""Capture UI golden renderer output and compare pixels without updating baselines."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import platform
import struct
import subprocess
import sys
import zlib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests/ui"))
sys.path.insert(0, str(ROOT / "tests/editor"))
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "tests"))

import golden_diff  # noqa: E402


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _git(*args: str) -> str | None:
    try:
        return subprocess.run(["git", *args], cwd=ROOT, check=True, text=True,
                              capture_output=True, timeout=5).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def _command(*args: str) -> str | None:
    try:
        return subprocess.run(list(args), cwd=ROOT, check=True, text=True,
                              capture_output=True, timeout=8).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def _display_scale_summary() -> dict[str, Any]:
    keys = ("Resolution:", "UI Looks like:", "Main Display:", "Mirror:", "Online:",
            "Retina:", "Refresh Rate:")
    try:
        result = subprocess.run(["system_profiler", "SPDisplaysDataType"], cwd=ROOT,
                                check=True, text=True, capture_output=True, timeout=10)
        lines = [line.strip() for line in result.stdout.splitlines()
                 if any(key in line for key in keys)]
        return {"available": True, "display_count_hint": sum(line.startswith("Resolution:") for line in lines),
                "scale_lines": lines}
    except (OSError, subprocess.SubprocessError) as exc:
        return {"available": False, "reason": type(exc).__name__}


def _decode_png(data: bytes) -> tuple[int, int, bytes, list[dict[str, Any]]]:
    """Decode RGBA pixels while recording and omitting valid ancillary PNG chunks."""
    if len(data) > golden_diff.MAX_IMAGE_BYTES or not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("invalid PNG signature or image exceeds 64 MiB")
    chunks: list[tuple[bytes, bytes]] = []
    ancillary: list[dict[str, Any]] = []
    offset = 8
    saw_header = saw_data = saw_end = ended_data = False
    while offset < len(data):
        if len(data) - offset < 12 or len(chunks) >= 4096:
            raise ValueError("truncated PNG chunk or more than 4096 chunks")
        length = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4:offset + 8]
        end = offset + 12 + length
        if end > len(data):
            raise ValueError("PNG chunk exceeds input length")
        payload = data[offset + 8:offset + 8 + length]
        crc = struct.unpack_from(">I", data, offset + 8 + length)[0]
        if zlib.crc32(kind + payload) & 0xFFFFFFFF != crc:
            raise ValueError(f"bad PNG {kind.decode('ascii', 'replace')} CRC")
        if not saw_header:
            if kind != b"IHDR" or length != 13:
                raise ValueError("PNG must start with one 13-byte IHDR")
            saw_header = True
            chunks.append((kind, payload))
            offset = end
            continue
        if kind == b"IDAT":
            if ended_data:
                raise ValueError("PNG IDAT chunks are not consecutive")
            saw_data = True
            chunks.append((kind, payload))
        elif kind == b"IEND":
            if length or not saw_data or saw_end or end != len(data):
                raise ValueError("malformed PNG IEND or trailing bytes")
            saw_end = True
            chunks.append((kind, payload))
            break
        elif kind == b"IHDR":
            raise ValueError("duplicate PNG IHDR")
        elif saw_data:
            ended_data = True
            if kind[0] & 0x20 == 0:
                raise ValueError(f"unsupported critical PNG chunk {kind!r}")
            ancillary.append({"type": kind.decode("ascii", "replace"),
                              "length": length, "payload_sha256": _sha(payload)})
        else:
            if kind[0] & 0x20 == 0:
                raise ValueError(f"unsupported critical PNG chunk {kind!r}")
            ancillary.append({"type": kind.decode("ascii", "replace"),
                              "length": length, "payload_sha256": _sha(payload)})
        if kind == b"IHDR":
            chunks.append((kind, payload))
        offset = end
    if not saw_header or not saw_data or not saw_end:
        raise ValueError("incomplete PNG")

    def png_chunk(kind: bytes, payload: bytes) -> bytes:
        body = kind + payload
        return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    stripped = b"\x89PNG\r\n\x1a\n" + b"".join(
        png_chunk(kind, payload) for kind, payload in chunks
    )
    width, height, rgba = golden_diff.decode_png(stripped)
    return width, height, rgba, ancillary


def _compare(expected_png: bytes, actual_png: bytes) -> dict[str, Any]:
    ew, eh, expected, expected_chunks = _decode_png(expected_png)
    aw, ah, actual, actual_chunks = _decode_png(actual_png)
    metrics: dict[str, Any] = {
        "expected_dimensions": {"width": ew, "height": eh},
        "actual_dimensions": {"width": aw, "height": ah},
        "dimensions_equal": (ew, eh) == (aw, ah),
        "expected_ancillary_png_chunks": expected_chunks,
        "actual_ancillary_png_chunks": actual_chunks,
        "channel_metric_note": "Compares stored RGBA channel values; ICC/profile transforms are not applied.",
    }
    if (ew, eh) != (aw, ah):
        metrics.update({"pixel_samples_equal": False, "differing_pixels": None,
                        "differing_channels": None, "maximum_channel_delta": None,
                        "mean_absolute_channel_delta": None, "bounding_box": None})
        return metrics

    differing_pixels = differing_channels = max_delta = total_delta = 0
    left, top, right, bottom = ew, eh, -1, -1
    for pixel in range(ew * eh):
        offset = pixel * 4
        delta = [abs(expected[offset + channel] - actual[offset + channel])
                 for channel in range(4)]
        peak = max(delta)
        if peak:
            x, y = pixel % ew, pixel // ew
            left, top, right, bottom = min(left, x), min(top, y), max(right, x), max(bottom, y)
            differing_pixels += 1
            differing_channels += sum(value != 0 for value in delta)
            max_delta = max(max_delta, peak)
            total_delta += sum(delta)
    metrics.update({
        "pixel_samples_equal": differing_pixels == 0,
        "exact_equal_pixels": ew * eh - differing_pixels,
        "differing_pixels": differing_pixels,
        "differing_channels": differing_channels,
        "maximum_channel_delta": max_delta,
        "mean_absolute_channel_delta": total_delta / (ew * eh * 4),
        "bounding_box": None if right < 0 else {
            "left": left, "top": top, "right": right, "bottom": bottom,
            "width": right - left + 1, "height": bottom - top + 1,
        },
    })
    return metrics


def _heatmap(expected_png: bytes, actual_png: bytes) -> bytes | None:
    ew, eh, expected, _ = _decode_png(expected_png)
    aw, ah, actual, _ = _decode_png(actual_png)
    if (ew, eh) != (aw, ah):
        return None
    pixels = bytearray(ew * eh * 4)
    for pos in range(0, len(pixels), 4):
        if expected[pos:pos + 4] != actual[pos:pos + 4]:
            pixels[pos:pos + 4] = b"\xff\x20\x20\xff"
    return golden_diff.encode_png(ew, eh, bytes(pixels))


def _box_downsample_2x(png: bytes) -> bytes:
    width, height, rgba, _ = _decode_png(png)
    if width % 2 or height % 2:
        raise ValueError("2x diagnostic downsample requires even dimensions")
    out_width, out_height = width // 2, height // 2
    out = bytearray(out_width * out_height * 4)
    for y in range(out_height):
        for x in range(out_width):
            dst = (y * out_width + x) * 4
            src0 = ((y * 2) * width + x * 2) * 4
            src1 = src0 + width * 4
            for channel in range(4):
                total = (rgba[src0 + channel] + rgba[src0 + 4 + channel] +
                         rgba[src1 + channel] + rgba[src1 + 4 + channel])
                out[dst + channel] = (total + 2) // 4
    return golden_diff.encode_png(out_width, out_height, bytes(out))


def _capture(output: Path, category: str, name: str, expected_path: Path,
             actual_png: bytes, executable: Path) -> dict[str, Any]:
    if not expected_path.is_file():
        row: dict[str, Any] = {
            "category": category,
            "name": name,
            "expected_path": expected_path.relative_to(ROOT).as_posix(),
            "expected_png_sha256": None,
            "actual_png_sha256": _sha(actual_png),
            "png_byte_equal": None,
            "renderer_path": executable.relative_to(ROOT).as_posix(),
            "renderer_sha256": _sha(executable.read_bytes()),
            "metrics": {
                "actual_dimensions": {
                    "width": struct.unpack_from(">I", actual_png, 16)[0],
                    "height": struct.unpack_from(">I", actual_png, 20)[0],
                },
                "baseline_available": False,
            },
            "result": "unbaselined",
        }
        stem = f"{category}/{Path(name).stem}"
        actual_path = output / "actual" / f"{stem}.png"
        actual_path.parent.mkdir(parents=True, exist_ok=True)
        actual_path.write_bytes(actual_png)
        row["actual_image"] = actual_path.relative_to(output).as_posix()
        return row

    expected_png = expected_path.read_bytes()
    if actual_png == expected_png:
        width, height = struct.unpack_from(">II", expected_png, 16)
        metrics = {
            "expected_dimensions": {"width": width, "height": height},
            "actual_dimensions": {"width": width, "height": height},
            "dimensions_equal": True,
            "pixel_samples_equal": True,
            "exact_equal_pixels": width * height,
            "differing_pixels": 0,
            "differing_channels": 0,
            "maximum_channel_delta": 0,
            "mean_absolute_channel_delta": 0,
            "bounding_box": None,
            "metrics_note": "PNG bytes match exactly; pixel decoding was unnecessary.",
        }
    else:
        metrics = _compare(expected_png, actual_png)
        expected_width, expected_height = struct.unpack_from(">II", expected_png, 16)
        actual_width, actual_height = struct.unpack_from(">II", actual_png, 16)
        if (actual_width, actual_height) == (expected_width * 2, expected_height * 2):
            normalized = _box_downsample_2x(actual_png)
            normalized_metrics = _compare(expected_png, normalized)
            normalized_metrics["normalization"] = (
                "2x2 box average from the retained 2x actual raster to the expected pixel grid; "
                "diagnostic only, does not change the verifier result."
            )
            metrics["normalized_2x_comparison"] = normalized_metrics
    row: dict[str, Any] = {
        "category": category,
        "name": name,
        "expected_path": expected_path.relative_to(ROOT).as_posix(),
        "expected_png_sha256": _sha(expected_png),
        "actual_png_sha256": _sha(actual_png),
        "png_byte_equal": expected_png == actual_png,
        "renderer_path": executable.relative_to(ROOT).as_posix(),
        "renderer_sha256": _sha(executable.read_bytes()),
        "metrics": metrics,
        "result": "pass" if expected_png == actual_png else "fail",
    }
    stem = f"{category}/{Path(name).stem}"
    actual_path = output / "actual" / f"{stem}.png"
    actual_path.parent.mkdir(parents=True, exist_ok=True)
    actual_path.write_bytes(actual_png)
    row["actual_image"] = actual_path.relative_to(output).as_posix()
    if row["result"] == "fail":
        diff = _heatmap(expected_png, actual_png)
        if diff is not None:
            diff_path = output / "diff" / f"{stem}.png"
            diff_path.parent.mkdir(parents=True, exist_ok=True)
            diff_path.write_bytes(diff)
            row["difference_image"] = diff_path.relative_to(output).as_posix()
        normalized = metrics.get("normalized_2x_comparison")
        if normalized is not None:
            normalized_png = _box_downsample_2x(actual_png)
            normalized_path = output / "normalized" / f"{stem}.png"
            normalized_path.parent.mkdir(parents=True, exist_ok=True)
            normalized_path.write_bytes(normalized_png)
            row["normalized_actual_image"] = normalized_path.relative_to(output).as_posix()
            normalized_diff = _heatmap(expected_png, normalized_png)
            if normalized_diff is not None:
                normalized_diff_path = output / "diff-normalized" / f"{stem}.png"
                normalized_diff_path.parent.mkdir(parents=True, exist_ok=True)
                normalized_diff_path.write_bytes(normalized_diff)
                row["normalized_difference_image"] = normalized_diff_path.relative_to(output).as_posix()
    return row


def _metadata() -> dict[str, Any]:
    scale = _command("defaults", "read", "-g", "AppleInterfaceScaleFactor")
    return {
        "source_commit": _git("rev-parse", "HEAD"),
        "source_branch": _git("branch", "--show-current"),
        "os": platform.platform(),
        "macos_version": _command("sw_vers", "-productVersion"),
        "macos_build": _command("sw_vers", "-buildVersion"),
        "architecture": platform.machine(),
        "apple_interface_scale_factor": scale,
        "display_scale_summary": _display_scale_summary(),
        "display_scale_note": "Renderer paths are offscreen/headless; display data is context, not a proven cause.",
    }


def capture(output: Path, build: Path) -> dict[str, Any]:
    native = importlib.import_module("verify_native_shell_goldens")
    style = importlib.import_module("verify_style_goldens")
    chrome = importlib.import_module("verify_chrome")
    snap = importlib.import_module("verify_snap_overlay_goldens")
    accessibility = importlib.import_module("verify_accessibility_localization")

    bins = {
        "native": build / "bin/native_ui_render",
        "style": build / "bin/viewer_chrome_render",
        "snap": build / "bin/editor_snap_overlay_render",
        "accessibility": build / "bin/editor_chrome_render",
    }
    for path in bins.values():
        if not path.is_file():
            raise FileNotFoundError(f"missing renderer executable: {path}")
    rows: list[dict[str, Any]] = []
    style_major, style_profile = style.profile_dir()
    chrome_major, chrome_profile = chrome.profile_dir()
    snap_major, snap_profile = snap.profile_dir()
    access_major, access_profile = accessibility.profile_dir()

    recent = Path("../../tests/svg/shape/fixtures/rect.svg")
    icon = ROOT / "app/Vexel-Icon-1024.png"
    old_cwd = Path.cwd()
    try:
        os.chdir(build)
        for screen in native.SCREENS:
            for theme in native.THEMES:
                name = f"{screen}-{theme}.png"
                resource = icon if screen == "app-icon" else recent
                actual = native.render(bins["native"], screen, theme, resource)
                major, profile = native.os_profile()
                rows.append(_capture(output, f"native-shell-{profile}", name,
                                     native.GOLDEN_DIR / profile / name,
                                     actual, bins["native"]))
    finally:
        os.chdir(old_cwd)

    for width, height in style.SIZES:
        for theme in style.THEMES:
            for state in style.STATES:
                viewport = style.layout_width(width, state)
                name = f"editor-{theme}-{width}x{height}-{state}.png"
                actual = style.render(bins["style"], theme, width, height, state, viewport)
                rows.append(_capture(output, f"editor-style-macos-{style_major}", name,
                                     style_profile / name, actual, bins["style"]))

    for theme in ("dark", "light"):
        name = f"editor-chrome-{theme}.png"
        actual = chrome.render(bins["accessibility"], theme)
        rows.append(_capture(output, f"editor-chrome-macos-{chrome_major}", name,
                             chrome_profile / name, actual, bins["accessibility"]))

    for theme in ("dark", "light"):
        name = f"snap-overlay-{theme}.png"
        actual = snap.render(bins["snap"], theme)
        rows.append(_capture(output, f"snap-overlay-macos-{snap_major}", name,
                             snap_profile / name, actual, bins["snap"]))

    for theme in ("dark", "light"):
        name = f"accessibility-chrome-{theme}-{accessibility.INTERFACE_SCALE}.png"
        actual = accessibility.render(bins["accessibility"], theme)
        rows.append(_capture(output, f"accessibility-localization-macos-{access_major}", name,
                             access_profile / name, actual, bins["accessibility"]))

    report = {
        "schema_version": 1,
        "purpose": "Diagnostic image comparison only; expected UI goldens and checks are read-only.",
        "comparison_policy": "Captures every image in all five renderer matrices even after a mismatch; this is diagnostic coverage beyond each verifier's fail-fast behavior.",
        "metadata": _metadata(),
        "summary": {
            "images_compared": len(rows),
            "exact_matches": sum(row["result"] == "pass" for row in rows),
            "mismatches": sum(row["result"] == "fail" for row in rows),
            "unbaselined": sum(row["result"] == "unbaselined" for row in rows),
            "categories": {},
        },
        "images": rows,
    }
    for category in sorted({row["category"] for row in rows}):
        items = [row for row in rows if row["category"] == category]
        report["summary"]["categories"][category] = {
            "compared": len(items),
            "mismatches": sum(row["result"] == "fail" for row in items),
            "unbaselined": sum(row["result"] == "unbaselined" for row in items),
            "mismatch_names": [row["name"] for row in items if row["result"] == "fail"],
        }
    report_path = output / "report.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True,
                        help="new output directory under repository .cache")
    parser.add_argument("--build-dir", type=Path, default=ROOT / "build/mac-debug",
                        help="configured Debug build directory")
    args = parser.parse_args()
    output = args.output if args.output.is_absolute() else ROOT / args.output
    output = output.resolve()
    cache = (ROOT / ".cache").resolve()
    if not output.is_relative_to(cache) or output == cache:
        parser.error("--output must be a new directory below repository .cache/")
    if output.exists():
        parser.error("--output must name a new directory")
    output.mkdir(parents=True)
    try:
        report = capture(output, args.build_dir.resolve())
    except Exception as exc:
        print(f"UI golden audit capture failed: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(report["summary"], sort_keys=True))
    return 1 if (report["summary"]["mismatches"] or
                 report["summary"]["unbaselined"]) else 0


if __name__ == "__main__":
    raise SystemExit(main())
