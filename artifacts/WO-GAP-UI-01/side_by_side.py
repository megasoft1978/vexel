#!/usr/bin/env python3
"""Stdlib-only before/after PNG comparison writer (before left, after right, same scale).

Usage:
  side_by_side.py blob <path> <before-rev> <after-sha256-prefix> <out.png>
      before = blob of <path> at <before-rev>; after = the blob of <path> in any
      commit whose SHA-256 starts with <after-sha256-prefix>.
  side_by_side.py file <before-png> <after-png> <out.png>
"""
import hashlib
import struct
import subprocess
import sys
import zlib

ROOT = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                      text=True, check=True).stdout.strip()


def git_bytes(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, check=True).stdout


def decode_png(data):
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, header = 8, b"", None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
    width, height, depth, color, _, _, interlace = header
    if depth != 8 or color not in (2, 6) or interlace != 0:
        raise ValueError("only 8-bit RGB/RGBA non-interlaced PNGs are supported")
    channels = 4 if color == 6 else 3
    stride = width * channels
    raw = zlib.decompress(idat)
    rows, prev = [], bytearray(stride)
    for y in range(height):
        base = y * (stride + 1)
        kind, line = raw[base], bytearray(raw[base + 1:base + 1 + stride])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + a) & 255
            elif kind == 2:
                line[i] = (line[i] + b) & 255
            elif kind == 3:
                line[i] = (line[i] + ((a + b) >> 1)) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 255
        rows.append(bytes(line))
        prev = line
    if channels == 3:
        rows = [b"".join(r[i:i + 3] + b"\xff" for i in range(0, len(r), 3)) for r in rows]
    return width, height, rows


def chunk(kind, body):
    return struct.pack(">I", len(body)) + kind + body + struct.pack(
        ">I", zlib.crc32(kind + body) & 0xFFFFFFFF)


def write_png(path, width, height, rows):
    raw = b"".join(b"\0" + r for r in rows)
    with open(path, "wb") as handle:
        handle.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(
            ">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b""))


def compose(before, after, out):
    bw, bh, brows = decode_png(before)
    aw, ah, arows = decode_png(after)
    if (bw, bh) != (aw, ah):
        raise SystemExit(f"size mismatch {bw}x{bh} vs {aw}x{ah}: not the same scale")
    gap = b"\x80\x80\x80\xff" * 4
    write_png(out, bw * 2 + 4, bh, [b + gap + a for b, a in zip(brows, arows)])
    print(f"{out}: {bw}x{bh} before (left) | after (right), same scale")


def find_after(path, prefix):
    revs = git_bytes("log", "--all", "--format=%H", "--", path).decode().split()
    for rev in revs:
        try:
            blob = git_bytes("show", f"{rev}:{path}")
        except subprocess.CalledProcessError:
            continue
        if hashlib.sha256(blob).hexdigest().startswith(prefix):
            return blob
    return None


def main(argv):
    if len(argv) == 6 and argv[1] == "blob":
        _, _, path, rev, prefix, out = argv
        before = git_bytes("show", f"{rev}:{path}")
        after = find_after(path, prefix)
        if after is None:
            print(f"after blob not found for {path} {prefix}")
            return 2
        compose(before, after, out)
        return 0
    if len(argv) == 5 and argv[1] == "file":
        compose(open(argv[2], "rb").read(), open(argv[3], "rb").read(), argv[4])
        return 0
    print(__doc__)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
