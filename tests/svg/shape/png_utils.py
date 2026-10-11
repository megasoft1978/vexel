"""PNG decoding helpers shared by renderer differential tests."""
import struct
import zlib


def png_rgba(data):
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not PNG")
    pos, compressed, width, height, bpp = 8, bytearray(), 0, 0, 0
    while pos < len(data):
        size = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        payload = data[pos + 8:pos + 8 + size]
        if kind == b"IHDR":
            width, height, depth, color, comp, filt, interlace = struct.unpack(">IIBBBBB", payload)
            if depth != 8 or color not in (2, 6) or interlace != 0:
                raise ValueError("reference PNG must be noninterlaced RGB8 or RGBA8")
            bpp = 3 if color == 2 else 4
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break
        pos += size + 12
    raw = zlib.decompress(compressed)
    stride = width * bpp
    rows, previous, off = [], bytearray(stride), 0
    for _ in range(height):
        f = raw[off]
        row = bytearray(raw[off + 1:off + 1 + stride])
        off += stride + 1
        for i in range(stride):
            a = row[i - bpp] if i >= bpp else 0
            b = previous[i]
            c = previous[i - bpp] if i >= bpp else 0
            if f == 1:
                row[i] = (row[i] + a) & 255
            elif f == 2:
                row[i] = (row[i] + b) & 255
            elif f == 3:
                row[i] = (row[i] + ((a + b) // 2)) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                predictor = a if pa <= pb and pa <= pc else b if pb <= pc else c
                row[i] = (row[i] + predictor) & 255
            elif f != 0:
                raise ValueError("unknown PNG filter")
        rows.append(row)
        previous = row
    pixels = b"".join(rows)
    if bpp == 4:
        return width, height, pixels
    rgba = bytearray(width * height * 4)
    for pixel in range(width * height):
        source = pixel * 3
        target = pixel * 4
        rgba[target:target + 3] = pixels[source:source + 3]
        rgba[target + 3] = 255
    return width, height, bytes(rgba)

