#!/usr/bin/env python3
"""Generate tiny deterministic platform-media fixtures without downloads."""

from __future__ import annotations

import argparse
import binascii
import pathlib
import struct
import subprocess
import zlib


def png_chunk(name: bytes, data: bytes) -> bytes:
    checksum = binascii.crc32(name + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + name + data + struct.pack(">I", checksum)


def write_png(path: pathlib.Path, width: int, height: int, pixels: bytes) -> None:
    row_bytes = width * 4
    if len(pixels) != row_bytes * height:
        raise ValueError("pixel buffer size does not match PNG dimensions")
    rows = b"".join(b"\0" + pixels[y * row_bytes : (y + 1) * row_bytes] for y in range(height))
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", header)
        + png_chunk(b"IDAT", zlib.compress(rows, 9))
        + png_chunk(b"IEND", b"")
    )


def write_solid_png(path: pathlib.Path, width: int, height: int) -> None:
    compressor = zlib.compressobj(9)
    compressed_parts = []
    row = b"\0" + bytes(width * 4)
    for _ in range(height):
        chunk = compressor.compress(row)
        if chunk:
            compressed_parts.append(chunk)
    compressed_parts.append(compressor.flush())
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", header)
        + png_chunk(b"IDAT", b"".join(compressed_parts))
        + png_chunk(b"IEND", b"")
    )


def gif_frame(index: int) -> bytes:
    # A one-pixel GIF image: clear, palette index, end; codes are LSB-first.
    code_bits = (4, index, 5)
    packed = 0
    bit_offset = 0
    for code in code_bits:
        packed |= code << bit_offset
        bit_offset += 3
    compressed = bytes((packed & 0xFF, (packed >> 8) & 0xFF))
    return (
        b"\x2c\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02"
        + bytes((len(compressed),))
        + compressed
        + b"\x00"
    )


def write_gif(path: pathlib.Path, animated: bool) -> None:
    header = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff"
    data = header + gif_frame(0)
    if animated:
        data += gif_frame(1)
    path.write_bytes(data + b"\x3b")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", required=True, type=pathlib.Path)
    args = parser.parse_args()
    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    png_pixels = bytes(
        (
            255, 0, 0, 255,  # opaque red
            0, 255, 0, 128,  # half-transparent green
            0, 0, 255, 0,    # transparent blue
            10, 20, 30, 255,
            50, 60, 70, 255,
            90, 100, 110, 255,
        )
    )
    write_png(output_dir / "sample.png", 3, 2, png_pixels)
    opaque_pixels = bytes(
        (255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255,
         255, 255, 0, 255, 0, 255, 255, 255, 255, 0, 255, 255)
    )
    source_png = output_dir / "jpeg-source.png"
    write_png(source_png, 3, 2, opaque_pixels)
    subprocess.run(
        ["/usr/bin/sips", "-s", "format", "jpeg", "-s", "formatOptions", "90",
         str(source_png), "--out", str(output_dir / "sample.jpg")],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    source_png.unlink()
    write_gif(output_dir / "static.gif", animated=False)
    write_gif(output_dir / "animated.gif", animated=True)
    write_solid_png(output_dir / "oversized-dimension.png", 16385, 1)
    write_solid_png(output_dir / "oversized-pixels.png", 8192, 8193)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
