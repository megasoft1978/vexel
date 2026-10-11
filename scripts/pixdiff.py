#!/usr/bin/env python3
"""Precise image comparison for PPM(P6)/PNG: pixdiff.py A B [--out diff.png] [--tolerance N]

Reports dimensions, differing-pixel count, bounding box, max/mean channel delta, the first differing
pixels with both colours, and writes a highlight image (differences in red over a dimmed copy of A).
Exit 0 when identical within tolerance, 1 otherwise.
"""
from __future__ import annotations

import argparse
import struct
import sys
import tempfile
import zlib
from pathlib import Path


def read_png(data: bytes) -> tuple[int, int, bytes]:
    """Minimal PNG reader: 8-bit RGB/RGBA/gray(+alpha), non-interlaced; alpha is dropped over white."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit("not a PNG or P6 PPM file")
    pos, idat, width = 8, [], 0
    height = depth = ctype = interlace = 0
    while pos < len(data):
        length, tag = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if tag == b"IHDR":
            width, height, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
        elif tag == b"IDAT":
            idat.append(body)
    if depth != 8 or interlace or ctype not in (0, 2, 4, 6):
        raise SystemExit(f"unsupported PNG (depth {depth}, colour type {ctype}, interlace {interlace})")
    channels = {0: 1, 2: 3, 4: 2, 6: 4}[ctype]
    stride = width * channels
    raw = zlib.decompress(b"".join(idat))
    rows, prev = [], bytearray(stride)
    for y in range(height):
        offset = y * (stride + 1)
        kind, line = raw[offset], bytearray(raw[offset + 1:offset + 1 + stride])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + a) & 255
            elif kind == 2:
                line[i] = (line[i] + b) & 255
            elif kind == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif kind == 4:
                p_ = a + b - c
                pa, pb, pc = abs(p_ - a), abs(p_ - b), abs(p_ - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line)
        prev = line
    rgb = bytearray()
    for line in rows:
        for x in range(width):
            px = line[x * channels:(x + 1) * channels]
            if ctype == 0:
                rgb += bytes((px[0],) * 3)
            elif ctype == 4:
                al = px[1]
                v = (px[0] * al + 255 * (255 - al)) // 255
                rgb += bytes((v,) * 3)
            elif ctype == 2:
                rgb += bytes(px)
            else:
                al = px[3]
                rgb += bytes(((px[k] * al + 255 * (255 - al)) // 255) for k in range(3))
    return width, height, bytes(rgb)


def load(path: Path) -> tuple[int, int, bytes]:
    data = path.read_bytes()
    if not data.startswith(b"P6"):
        return read_png(data)
    parts, pos = [], 2
    while len(parts) < 3:
        while data[pos:pos + 1].isspace():
            pos += 1
        if data[pos:pos + 1] == b"#":
            pos = data.index(b"\n", pos)
            continue
        end = pos
        while not data[end:end + 1].isspace():
            end += 1
        parts.append(int(data[pos:end]))
        pos = end
    pos += 1
    w, h, _ = parts
    return w, h, data[pos:pos + w * h * 3]


def write_png(path: Path, w: int, h: int, rgb: bytes) -> None:
    raw = b"".join(b"\x00" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))

    def chunk(tag: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body))

    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("a", type=Path)
    ap.add_argument("b", type=Path)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--tolerance", type=int, default=0, help="max allowed per-channel delta")
    args = ap.parse_args()
    wa, ha, da = load(args.a)
    wb, hb, db = load(args.b)
    if (wa, ha) != (wb, hb):
        print(f"size differs: {wa}x{ha} vs {wb}x{hb}")
        return 1
    count = total = peak = 0
    x0, y0, x1, y1 = wa, ha, -1, -1
    first = []
    out = bytearray(len(da))
    for i in range(0, len(da), 3):
        ra, ga, ba = da[i], da[i + 1], da[i + 2]
        rb, gb, bb = db[i], db[i + 1], db[i + 2]
        delta = max(abs(ra - rb), abs(ga - gb), abs(ba - bb))
        if delta > args.tolerance:
            count += 1
            total += delta
            peak = max(peak, delta)
            px, py = (i // 3) % wa, (i // 3) // wa
            x0, y0, x1, y1 = min(x0, px), min(y0, py), max(x1, px), max(y1, py)
            if len(first) < 8:
                first.append((px, py, (ra, ga, ba), (rb, gb, bb)))
            out[i:i + 3] = b"\xff\x00\x00"
        else:
            out[i:i + 3] = bytes((ra // 2 + 64, ga // 2 + 64, ba // 2 + 64))
    pixels = wa * ha
    if count == 0:
        print(f"identical within tolerance {args.tolerance} ({wa}x{ha})")
        return 0
    print(f"{count} of {pixels} pixels differ ({100.0 * count / pixels:.3f}%), max delta {peak}, "
          f"mean delta {total / count:.1f}, bbox x{x0}..{x1} y{y0}..{y1}")
    for px, py, ca, cb in first:
        print(f"  ({px},{py}) {ca} vs {cb}")
    target = args.out or Path(tempfile.gettempdir()) / "pixdiff.png"
    write_png(target, wa, ha, bytes(out))
    print(f"highlight image: {target}")
    return 1


sys.exit(main())
