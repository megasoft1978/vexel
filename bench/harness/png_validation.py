"""Strict PNG structure checks for fixed-size benchmark output."""

from __future__ import annotations

import struct
import zlib

try:
    from bench.harness.errors import BenchError
except ModuleNotFoundError:
    from errors import BenchError

DIMENSION = 256
PNG_LIMIT = 2 * 1024 * 1024

def _png_pass_size(size: int, start: int, step: int) -> int:
    return 0 if size <= start else (size - start + step - 1) // step


def _png_rows(width: int, height: int, bit_depth: int, channels: int,
              interlace: int) -> list[tuple[int, int]]:
    passes = ((0, 0, 1, 1),) if interlace == 0 else (
        (0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4),
        (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2))
    rows = []
    for x, y, dx, dy in passes:
        pass_width = _png_pass_size(width, x, dx)
        pass_height = _png_pass_size(height, y, dy)
        if pass_width and pass_height:
            row_bytes = (pass_width * channels * bit_depth + 7) // 8
            rows.append((row_bytes, pass_height))
    return rows


def _png_decoded_size(rows: list[tuple[int, int]]) -> int:
    return sum((row_bytes + 1) * count for row_bytes, count in rows)


def png_dimensions(data: bytes) -> tuple[int, int]:
    if len(data) < 45 or data[:8] != b"\x89PNG\r\n\x1a\n":
        raise BenchError("output is not a PNG")
    offset, width, height = 8, 0, 0
    seen_ihdr, seen_plte, seen_idat = False, False, False
    idat_ended, seen_end = False, False
    bit_depth, color_type, interlace = 0, -1, 0
    compressed = bytearray()
    palette_size = 0
    while offset + 12 <= len(data):
        size = struct.unpack(">I", data[offset:offset + 4])[0]
        end = offset + 12 + size
        if size > PNG_LIMIT or end > len(data):
            raise BenchError("malformed PNG chunk bounds")
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + size]
        crc = struct.unpack(">I", data[offset + 8 + size:end])[0]
        if zlib.crc32(kind + payload) & 0xffffffff != crc:
            raise BenchError("malformed PNG chunk checksum")
        if not seen_ihdr:
            if kind != b"IHDR" or size != 13 or offset != 8:
                raise BenchError("malformed PNG header order")
            width, height = struct.unpack(">II", payload[:8])
            if (width, height) != (DIMENSION, DIMENSION):
                raise BenchError("output PNG dimensions differ from 256x256 task")
            bit_depth, color_type = payload[8], payload[9]
            interlace = payload[12]
            valid_depths = {0: (1, 2, 4, 8, 16), 2: (8, 16),
                            3: (1, 2, 4, 8), 4: (8, 16), 6: (8, 16)}
            if (color_type not in valid_depths or bit_depth not in valid_depths[color_type]
                    or payload[10:12] != b"\0\0" or interlace not in (0, 1)):
                raise BenchError("unsupported PNG header format")
            seen_ihdr = True
        elif kind == b"IHDR":
            raise BenchError("duplicate PNG header")
        elif kind == b"PLTE":
            if (seen_plte or seen_idat or not size or size > 768 or size % 3
                    or color_type in (0, 4)):
                raise BenchError("malformed PNG palette")
            palette_size = size // 3
            if color_type == 3 and palette_size > (1 << bit_depth):
                raise BenchError("PNG palette exceeds indexed bit depth")
            seen_plte = True
        if kind == b"IDAT":
            if color_type == 3 and not seen_plte:
                raise BenchError("indexed PNG is missing its palette")
            if idat_ended:
                raise BenchError("PNG image data chunks are not contiguous")
            seen_idat = True
            compressed.extend(payload)
        elif seen_idat and kind != b"IEND":
            idat_ended = True
        if kind == b"IEND":
            if not seen_idat or size != 0 or end != len(data):
                raise BenchError("malformed PNG end")
            seen_end = True
            break
        if kind not in {b"IHDR", b"PLTE", b"IDAT"} and kind[0] & 0x20 == 0:
            raise BenchError("unsupported critical PNG chunk")
        offset = end
    if not seen_ihdr or not seen_end or not width or not height:
        raise BenchError("incomplete PNG")
    try:
        channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color_type]
        rows = _png_rows(width, height, bit_depth, channels, interlace)
        expected_size = _png_decoded_size(rows)
        inflater = zlib.decompressobj()
        decoded = inflater.decompress(bytes(compressed), expected_size + 1)
        if inflater.unconsumed_tail:
            raise BenchError("PNG pixel data exceeds declared dimensions")
        decoded += inflater.flush()
    except zlib.error as exc:
        raise BenchError("malformed PNG image data") from exc
    if not inflater.eof or inflater.unused_data or len(decoded) != expected_size:
        raise BenchError("malformed PNG pixel data size")
    offset = 0
    for row_bytes, count in rows:
        for _ in range(count):
            if decoded[offset] > 4:
                raise BenchError("malformed PNG row filter")
            offset += row_bytes + 1
    return width, height


def fake_png() -> bytes:
    def chunk(kind: bytes, payload: bytes) -> bytes:
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff)
    raw = b"".join(b"\0" + bytes([30, 90, 180, 255]) * DIMENSION for _ in range(DIMENSION))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", DIMENSION, DIMENSION, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")

