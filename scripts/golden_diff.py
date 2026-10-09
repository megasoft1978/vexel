#!/usr/bin/env python3
"""Compare a rendered RGBA PNG with one hash-pinned golden image."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import struct
import sys
import zlib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "tests/golden/manifest.lock"
SIGNATURE = b"\x89PNG\r\n\x1a\n"
MAX_IMAGE_BYTES = 64 * 1024 * 1024
MAX_PIXELS = 16_777_216
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


class CompareError(ValueError):
    """An input, metadata, PNG, or output contract was violated."""


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _read_png(path: Path) -> bytes:
    """Read at most one byte beyond the image limit to bound input memory."""
    try:
        with path.open("rb") as handle:
            data = handle.read(MAX_IMAGE_BYTES + 1)
    except OSError as exc:
        raise CompareError(f"cannot read PNG input: {exc}") from exc
    if len(data) > MAX_IMAGE_BYTES:
        raise CompareError("PNG exceeds the 64 MiB input limit")
    return data


def _read_manifest(path: Path | None = None) -> dict[str, dict[str, Any]]:
    path = MANIFEST if path is None else path
    try:
        lock = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise CompareError(f"cannot read golden manifest: {exc}") from exc
    fixtures = lock.get("fixtures") if isinstance(lock, dict) else None
    if not isinstance(fixtures, list):
        raise CompareError("golden manifest has no fixture list")
    result: dict[str, dict[str, Any]] = {}
    for entry in fixtures:
        if not isinstance(entry, dict) or not isinstance(entry.get("fixture_id"), str):
            raise CompareError("malformed fixture entry in golden manifest")
        fixture_id = entry["fixture_id"]
        if fixture_id in result:
            raise CompareError(f"duplicate fixture ID {fixture_id}")
        if not SHA256_RE.fullmatch(str(entry.get("png_sha256", ""))) or not SHA256_RE.fullmatch(
            str(entry.get("source_sha256", ""))
        ):
            raise CompareError(f"invalid pinned hash for fixture {fixture_id}")
        result[fixture_id] = entry
    return result


def _unfilter(kind: int, row: bytes, previous: bytes, bpp: int) -> bytes:
    out = bytearray(row)
    for i, value in enumerate(row):
        left = out[i - bpp] if i >= bpp else 0
        above = previous[i]
        upper_left = previous[i - bpp] if i >= bpp else 0
        if kind == 1:
            value += left
        elif kind == 2:
            value += above
        elif kind == 3:
            value += (left + above) // 2
        elif kind == 4:
            p = left + above - upper_left
            pa, pb, pc = abs(p - left), abs(p - above), abs(p - upper_left)
            predictor = left if pa <= pb and pa <= pc else above if pb <= pc else upper_left
            value += predictor
        elif kind != 0:
            raise CompareError(f"unsupported PNG filter {kind}")
        out[i] = value & 0xFF
    return bytes(out)


def decode_png(data: bytes) -> tuple[int, int, bytes]:
    """Decode PNG color type 6, bit depth 8, non-interlaced, with CRC checks."""
    if len(data) > MAX_IMAGE_BYTES:
        raise CompareError("PNG exceeds the 64 MiB input limit")
    if not data.startswith(SIGNATURE):
        raise CompareError("invalid PNG signature")
    offset = len(SIGNATURE)
    width = height = None
    idat: list[bytes] = []
    seen_idat = False
    ended_idat = False
    seen_iend = False
    seen_bkgd = False
    chunk_count = 0
    while offset < len(data):
        if len(data) - offset < 12:
            raise CompareError("truncated PNG chunk")
        length = struct.unpack_from(">I", data, offset)[0]
        chunk_type = data[offset + 4 : offset + 8]
        end = offset + 12 + length
        if end > len(data):
            raise CompareError("PNG chunk length exceeds file")
        payload = data[offset + 8 : offset + 8 + length]
        crc = struct.unpack_from(">I", data, offset + 8 + length)[0]
        if zlib.crc32(chunk_type + payload) & 0xFFFFFFFF != crc:
            raise CompareError(f"bad CRC in {chunk_type.decode('ascii', 'replace')} chunk")
        chunk_count += 1
        if chunk_count > 4096:
            raise CompareError("PNG has too many chunks")
        if chunk_type == b"IHDR":
            if width is not None or chunk_count != 1 or length != 13:
                raise CompareError("malformed PNG IHDR")
            width, height, depth, color, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", payload
            )
            if not width or not height or width * height > MAX_PIXELS:
                raise CompareError("PNG dimensions are zero or exceed the pixel limit")
            if (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise CompareError("unsupported PNG encoding; expected non-interlaced 8-bit RGBA")
        elif chunk_type == b"IDAT":
            if width is None or ended_idat:
                raise CompareError("PNG IDAT chunks are misplaced")
            seen_idat = True
            idat.append(payload)
        elif chunk_type == b"IEND":
            if length or not seen_idat or seen_iend:
                raise CompareError("malformed PNG IEND")
            seen_iend = True
            offset = end
            if offset != len(data):
                raise CompareError("trailing data after PNG IEND")
            break
        else:
            if not seen_idat:
                if chunk_type not in (b"bKGD",):
                    raise CompareError(f"unsupported PNG chunk {chunk_type.decode('ascii', 'replace')}")
            else:
                ended_idat = True
                if chunk_type != b"bKGD":
                    raise CompareError(f"unsupported PNG chunk {chunk_type.decode('ascii', 'replace')}")
                raise CompareError("PNG bKGD chunk must precede IDAT")
            if chunk_type == b"bKGD":
                if length != 6 or seen_bkgd:
                    raise CompareError("malformed or duplicate PNG bKGD")
                seen_bkgd = True
        offset = end
    if width is None or not seen_iend:
        raise CompareError("PNG is missing IHDR, IDAT, or IEND")
    assert height is not None
    expected_length = height * (width * 4 + 1)
    try:
        decoder = zlib.decompressobj()
        raw = decoder.decompress(b"".join(idat), expected_length + 1)
        if len(raw) > expected_length or decoder.unconsumed_tail:
            raise CompareError("PNG decompressed data exceeds dimensions")
        raw += decoder.flush()
    except zlib.error as exc:
        raise CompareError(f"invalid PNG compressed image data: {exc}") from exc
    if len(raw) != expected_length or not decoder.eof or decoder.unused_data or decoder.unconsumed_tail:
        raise CompareError("PNG decompressed data length does not match dimensions")
    rows: list[bytes] = []
    previous = bytes(width * 4)
    cursor = 0
    for _ in range(height):
        kind = raw[cursor]
        cursor += 1
        row = _unfilter(kind, raw[cursor : cursor + width * 4], previous, 4)
        rows.append(row)
        previous = row
        cursor += width * 4
    return width, height, b"".join(rows)


def _edge_pixels(expected: bytes, width: int, height: int) -> bytearray:
    edges = bytearray(width * height)
    for y in range(height):
        for x in range(width):
            pos = (y * width + x) * 4
            color = expected[pos : pos + 4]
            for ny in range(max(0, y - 1), min(height, y + 2)):
                for nx in range(max(0, x - 1), min(width, x + 2)):
                    if nx == x and ny == y:
                        continue
                    other = (ny * width + nx) * 4
                    if expected[other : other + 4] != color:
                        edges[y * width + x] = 1
                        break
                if edges[y * width + x]:
                    break
    return edges


def _output_path(raw: str, root: Path = ROOT) -> Path:
    path = Path(raw)
    if not path.is_absolute():
        path = root / path
    # Normalize dot-dot components before checking the cache boundary. Path.is_relative_to
    # is lexical, so checking an unnormalized `.cache/../...` would accept an escape.
    absolute = Path(os.path.abspath(path))
    try:
        relative = absolute.relative_to(root.absolute())
    except ValueError as exc:
        raise CompareError("output paths must stay under repository .cache/") from exc
    cursor = root.absolute()
    for part in relative.parts:
        cursor = cursor / part
        if cursor.is_symlink():
            raise CompareError("output path must not contain symlinks")
    cache = (root / ".cache").absolute()
    if not absolute.is_relative_to(cache):
        raise CompareError("output paths must stay under repository .cache/")
    if absolute.exists():
        raise CompareError(f"refusing to overwrite existing output: {absolute}")
    return absolute


def _write_new(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        fd = os.open(path, flags, 0o600)
    except FileExistsError as exc:
        raise CompareError(f"refusing to overwrite existing output: {path}") from exc
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
    except BaseException:
        path.unlink(missing_ok=True)
        raise


def encode_png(width: int, height: int, rgba: bytes) -> bytes:
    def chunk(kind: bytes, payload: bytes) -> bytes:
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)

    raw = b"".join(b"\0" + rgba[y * width * 4 : (y + 1) * width * 4] for y in range(height))
    return SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(
        b"IDAT", zlib.compress(raw, level=9)
    ) + chunk(b"IEND", b"")


def _metadata(args: argparse.Namespace, fixture: dict[str, Any]) -> dict[str, Any]:
    for key in ("svg_sha256", "build_sha"):
        if not SHA256_RE.fullmatch(getattr(args, key)):
            raise CompareError(f"--{key.replace('_', '-')} must be 64 lowercase hex characters")
    if args.svg_sha256 != fixture["source_sha256"]:
        raise CompareError("SVG source SHA-256 does not match the pinned fixture identity")
    if not math.isfinite(args.scale) or not 0.01 <= args.scale <= 64:
        raise CompareError("--scale must be between 0.01 and 64")
    if type(args.frame) is not int or not 0 <= args.frame <= 1_000_000:
        raise CompareError("--frame must be between 0 and 1000000")
    for key in ("font", "backend"):
        value = getattr(args, key)
        if not value or len(value) > 256 or any(ord(ch) < 32 for ch in value):
            raise CompareError(f"--{key} must contain 1 to 256 printable characters")
    return {"svg_sha256": args.svg_sha256, "build_sha": args.build_sha,
            "scale": args.scale, "frame": args.frame, "font": args.font, "backend": args.backend}


def compare(args: argparse.Namespace) -> tuple[dict[str, Any], bytes]:
    fixtures = _read_manifest()
    if args.fixture not in fixtures:
        raise CompareError(f"unknown fixture ID {args.fixture}")
    fixture = fixtures[args.fixture]
    metadata = _metadata(args, fixture)
    expected_path = ROOT / fixture["png_path"]
    expected_png = _read_png(expected_path)
    actual_png = _read_png(Path(args.actual))
    expected_sha, actual_sha = _sha256(expected_png), _sha256(actual_png)
    if expected_sha != fixture["png_sha256"]:
        raise CompareError(f"frozen baseline hash mismatch for {args.fixture}")
    ew, eh, expected = decode_png(expected_png)
    aw, ah, actual = decode_png(actual_png)
    if (ew, eh) != (fixture["width"], fixture["height"]):
        raise CompareError("baseline dimensions do not match the frozen manifest")
    if (aw, ah) != (ew, eh):
        raise CompareError("actual image dimensions do not match the baseline")
    max_delta = total_delta = differing_channels = differing_pixels = 0
    tolerated = non_tolerated = 0
    edge = _edge_pixels(expected, ew, eh)
    heat = bytearray(ew * eh * 4)
    for pixel in range(ew * eh):
        offset = pixel * 4
        deltas = [abs(expected[offset + c] - actual[offset + c]) for c in range(4)]
        pixel_max = max(deltas)
        if pixel_max:
            differing_pixels += 1
            differing_channels += sum(delta != 0 for delta in deltas)
            max_delta = max(max_delta, pixel_max)
            total_delta += sum(deltas)
            x, y = pixel % ew, pixel // ew
            is_near_edge = False
            for ny in range(max(0, y - args.edge_radius), min(eh, y + args.edge_radius + 1)):
                for nx in range(max(0, x - args.edge_radius), min(ew, x + args.edge_radius + 1)):
                    if edge[ny * ew + nx]:
                        is_near_edge = True
                        break
                if is_near_edge:
                    break
            if pixel_max <= args.edge_channel_tolerance and is_near_edge:
                tolerated += 1
                heat[offset : offset + 4] = b"\xff\xff\x00\xff"
            else:
                non_tolerated += 1
                heat[offset : offset + 4] = b"\xff\x00\x00\xff"
    pixels = ew * eh
    report = {
        "schema_version": 1, "fixture_id": args.fixture,
        "expected_png_sha256": expected_sha, "actual_png_sha256": actual_sha,
        "pinned_svg_sha256": fixture["source_sha256"], "build_sha": metadata["build_sha"],
        "scale": metadata["scale"], "frame": metadata["frame"], "font": metadata["font"],
        "backend": metadata["backend"], "dimensions": {"width": ew, "height": eh},
        "tolerances": {"edge_radius": args.edge_radius,
                       "edge_channel_tolerance": args.edge_channel_tolerance},
        "metrics": {"exact_equal": differing_pixels == 0,
                    "exact_equal_pixels": pixels - differing_pixels,
                    "differing_pixels": differing_pixels,
                    "differing_channels": differing_channels,
                    "maximum_channel_delta": max_delta,
                    "total_absolute_channel_delta": total_delta,
                    "mean_absolute_channel_delta": total_delta / (pixels * 4)},
        "tolerance": {"tolerated_pixels": tolerated, "non_tolerated_pixels": non_tolerated},
        "result": "pass" if non_tolerated == 0 else "fail",
    }
    return report, encode_png(ew, eh, bytes(heat))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--actual", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument("--heatmap", required=True)
    parser.add_argument("--svg-sha256", dest="svg_sha256", required=True)
    parser.add_argument("--build-sha", dest="build_sha", required=True)
    parser.add_argument("--scale", type=float, required=True)
    parser.add_argument("--frame", type=int, required=True)
    parser.add_argument("--font", required=True)
    parser.add_argument("--backend", required=True)
    parser.add_argument("--edge-radius", type=int, choices=range(0, 3), default=1)
    parser.add_argument("--edge-channel-tolerance", type=int, choices=range(0, 17), default=1)
    args = parser.parse_args(argv)
    try:
        report_path = _output_path(args.report)
        heatmap_path = _output_path(args.heatmap)
        if report_path == heatmap_path:
            raise CompareError("report and heatmap paths must be different")
        report, heatmap = compare(args)
        report_bytes = (json.dumps(report, sort_keys=True, indent=2, ensure_ascii=True) + "\n").encode()
        _write_new(heatmap_path, heatmap)
        _write_new(report_path, report_bytes)
        print(report["result"])
        return 0 if report["result"] == "pass" else 1
    except (CompareError, OSError) as exc:
        print(f"golden diff error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
