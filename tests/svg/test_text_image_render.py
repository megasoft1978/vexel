"""Platform-backed text and image rendering differential checks."""
import base64
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib
from pathlib import Path

from tests.svg.shape.png_utils import png_rgba

ROOT = Path(__file__).resolve().parents[2]
TEMP_ROOT = ROOT / ".cache"
TEMP_ROOT.mkdir(exist_ok=True)
MEDIA_CLI = ROOT / "build/mac-debug/bin/shape_media_render_cli"


def _rgba_png_pixel(rgba):
    def chunk(kind, payload):
        body = kind + payload
        return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body))

    return (b"\x89PNG\r\n\x1a\n" +
            chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0)) +
            chunk(b"IDAT", zlib.compress(b"\x00" + bytes(rgba))) +
            chunk(b"IEND", b""))


def _alpha_bounds(rgba, width, height):
    points = [(offset // 4 % width, offset // 4 // width)
              for offset in range(3, len(rgba), 4) if rgba[offset] > 8]
    if not points:
        return None
    xs, ys = zip(*points)
    return min(xs), min(ys), max(xs), max(ys)


class TextImageRenderTests(unittest.TestCase):
    def test_coretext_text_render_tracks_librsvg_structure_with_font_tolerance(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by the text differential")
        self.assertTrue(MEDIA_CLI.is_file(), "run ./dev build before media differential tests")
        source = ('<svg width="72" height="40"><text x="36" y="30" '
                  'text-anchor="middle" font-family="sans-serif" font-size="24">'
                  'Vexel</text></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-text-diff-", dir=TEMP_ROOT) as temp:
            directory = Path(temp)
            input_path = directory / "text.svg"
            actual_path = directory / "actual.rgba"
            reference_path = directory / "reference.png"
            input_path.write_text(source, encoding="utf-8")
            actual_run = subprocess.run([str(MEDIA_CLI), str(input_path), str(actual_path)],
                                        capture_output=True, text=True)
            self.assertEqual(actual_run.returncode, 0, actual_run.stderr)
            subprocess.run([renderer, "--format", "png", "--output", str(reference_path),
                            str(input_path)], check=True, capture_output=True, text=True)
            width, height, expected = png_rgba(reference_path.read_bytes())
            actual = actual_path.read_bytes()
        self.assertEqual((width, height), (72, 40))
        self.assertEqual(len(actual), width * height * 4)
        actual_bounds = _alpha_bounds(actual, width, height)
        expected_bounds = _alpha_bounds(expected, width, height)
        self.assertIsNotNone(actual_bounds, "CoreText must produce outline pixels")
        self.assertIsNotNone(expected_bounds, "librsvg must produce text pixels")
        actual_width = actual_bounds[2] - actual_bounds[0] + 1
        expected_width = expected_bounds[2] - expected_bounds[0] + 1
        actual_height = actual_bounds[3] - actual_bounds[1] + 1
        expected_height = expected_bounds[3] - expected_bounds[1] + 1
        # CoreText and Pango can choose different sans-serif faces and edge coverage.
        # Keep the structural comparison tight enough to catch lost/misplaced runs.
        self.assertLessEqual(abs(actual_width - expected_width), 8)
        self.assertLessEqual(abs(actual_height - expected_height), 4)
        self.assertLessEqual(abs((actual_bounds[0] + actual_bounds[2]) -
                                 (expected_bounds[0] + expected_bounds[2])), 8)
        actual_ink = sum(actual[offset + 3] > 8 for offset in range(0, len(actual), 4))
        expected_ink = sum(expected[offset + 3] > 8 for offset in range(0, len(expected), 4))
        self.assertGreater(actual_ink, 0)
        self.assertGreater(expected_ink, 0)
        self.assertLessEqual(abs(actual_ink - expected_ink), max(32, expected_ink // 2))

    def test_embedded_image_render_matches_librsvg_through_clip_transform_and_opacity(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by the image differential")
        self.assertTrue(MEDIA_CLI.is_file(), "run ./dev build before media differential tests")
        encoded = base64.b64encode(_rgba_png_pixel((240, 80, 20, 255))).decode("ascii")
        source = (f'<svg width="24" height="20"><defs><clipPath id="crop" '
                  f'clipPathUnits="userSpaceOnUse"><rect x="5" y="4" width="4" '
                  f'height="4"/></clipPath></defs><g transform="translate(1 1)">'
                  f'<image x="2" y="2" width="8" height="8" opacity="0.5" '
                  f'clip-path="url(#crop)" href="data:image/png;base64,{encoded}"/>'
                  f'</g></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-image-diff-", dir=TEMP_ROOT) as temp:
            directory = Path(temp)
            input_path = directory / "image.svg"
            actual_path = directory / "actual.rgba"
            reference_path = directory / "reference.png"
            input_path.write_text(source, encoding="utf-8")
            actual_run = subprocess.run([str(MEDIA_CLI), str(input_path), str(actual_path)],
                                        capture_output=True, text=True)
            self.assertEqual(actual_run.returncode, 0, actual_run.stderr)
            subprocess.run([renderer, "--format", "png", "--output", str(reference_path),
                            str(input_path)], check=True, capture_output=True, text=True)
            width, height, expected = png_rgba(reference_path.read_bytes())
            actual = actual_path.read_bytes()
        self.assertEqual((width, height), (24, 20))
        self.assertEqual(len(actual), width * height * 4)
        for offset in range(0, len(actual), 4):
            self.assertEqual(actual[offset + 3], expected[offset + 3])
            self.assertLessEqual(max(abs(actual[offset + channel] -
                                          expected[offset + channel])
                                     for channel in range(3)), 1)
        pixels = [actual[offset:offset + 4] for offset in range(0, len(actual), 4)]
        colored = [pixel for pixel in pixels if pixel[3] != 0]
        self.assertEqual(len(colored), 16)
        self.assertTrue(all(pixel == bytes((240, 80, 20, 128)) for pixel in colored))



if __name__ == "__main__":
    unittest.main()
