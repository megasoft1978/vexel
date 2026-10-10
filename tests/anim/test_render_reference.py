"""Pixel spot checks against librsvg's static render of sampled values."""

from __future__ import annotations

from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib


ROOT = Path(__file__).resolve().parents[2]
ANIMATED_RENDER = ROOT / "build" / "mac-debug" / "bin" / "animation_render_cli"
FIXTURE = ROOT / "tests" / "anim" / "fixtures" / "slide-x.svg"


def decode_png(path: Path) -> tuple[int, int, bytes]:
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise AssertionError("rsvg-convert did not write a PNG")
    cursor = 8
    compressed = bytearray()
    width = height = color_type = bit_depth = interlace = 0
    while cursor + 12 <= len(data):
        length = struct.unpack_from(">I", data, cursor)[0]
        tag = data[cursor + 4:cursor + 8]
        payload_start = cursor + 8
        payload_end = payload_start + length
        if payload_end + 4 > len(data):
            raise AssertionError("truncated PNG chunk")
        payload = data[payload_start:payload_end]
        if tag == b"IHDR":
            width, height, bit_depth, color_type, _compression, _filter, interlace = struct.unpack(
                ">IIBBBBB", payload
            )
        elif tag == b"IDAT":
            compressed.extend(payload)
        elif tag == b"IEND":
            break
        cursor = payload_end + 4
    if bit_depth != 8 or color_type not in (2, 6) or interlace != 0:
        raise AssertionError("cross-check PNG uses an unsupported pixel encoding")
    channels = 4 if color_type == 6 else 3
    stride = width * channels
    raw = zlib.decompress(compressed)
    if len(raw) != height * (stride + 1):
        raise AssertionError("decoded PNG rows have an unexpected length")
    pixels = bytearray(width * height * 4)
    previous = bytearray(stride)
    input_offset = 0
    for y in range(height):
        filter_type = raw[input_offset]
        input_offset += 1
        row = bytearray(raw[input_offset:input_offset + stride])
        input_offset += stride
        for x in range(stride):
            left = row[x - channels] if x >= channels else 0
            above = previous[x]
            upper_left = previous[x - channels] if x >= channels else 0
            if filter_type == 1:
                row[x] = (row[x] + left) & 255
            elif filter_type == 2:
                row[x] = (row[x] + above) & 255
            elif filter_type == 3:
                row[x] = (row[x] + ((left + above) // 2)) & 255
            elif filter_type == 4:
                base = left + above - upper_left
                distances = (abs(base - left), abs(base - above), abs(base - upper_left))
                predictor = left if distances[0] <= distances[1] and distances[0] <= distances[2] else (
                    above if distances[1] <= distances[2] else upper_left
                )
                row[x] = (row[x] + predictor) & 255
            elif filter_type != 0:
                raise AssertionError("unsupported PNG row filter")
        for x in range(width):
            source = x * channels
            target = (y * width + x) * 4
            pixels[target:target + 3] = row[source:source + 3]
            pixels[target + 3] = row[source + 3] if channels == 4 else 255
        previous = row
    return width, height, bytes(pixels)


class RenderReferenceTests(unittest.TestCase):
    def test_sampled_animation_pixels_match_librsvg_static_values(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        with tempfile.TemporaryDirectory(prefix="anim-rsvg-") as temporary:
            directory = Path(temporary)
            for seconds in (0.0, 0.5, 1.0, 1.5):
                x = 1.0 + (39.0 - 1.0) * ((seconds % 2.0) / 2.0)
                static = directory / "static.svg"
                static.write_text(
                    f'<svg xmlns="http://www.w3.org/2000/svg" width="48" height="16">'
                    f'<rect x="{x:.9g}" y="4" width="8" height="8" fill="#2a9d8f"/>'
                    f'</svg>', encoding="utf-8"
                )
                actual = directory / "actual.rgba"
                expected = directory / "expected.png"
                subprocess.run(
                    [str(ANIMATED_RENDER), str(FIXTURE), f"{seconds:.17g}", str(actual)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                subprocess.run(
                    [str(converter), "--width", "48", "--height", "16", "--output", str(expected), str(static)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                width, height, oracle = decode_png(expected)
                self.assertEqual((width, height), (48, 16))
                render = actual.read_bytes()
                self.assertEqual(len(render), 48 * 16 * 4)
                x_probe = min(46, int(x) + 3)
                offset = (8 * width + x_probe) * 4
                self.assertEqual(render[offset:offset + 4], oracle[offset:offset + 4],
                                 f"pixel differs at t={seconds}")


if __name__ == "__main__":
    unittest.main()
