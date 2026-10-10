"""Headless viewer rendering and bounded CLI acceptance checks."""

from __future__ import annotations

import hashlib
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get("VEXEL_BINARY", ROOT / "build/mac-debug/bin/svgworkbench"))
PROJECT_FIXTURE = ROOT / "tests/svg/shape/fixtures/rect.svg"
SIMPLE_FIXTURE = ROOT / "tests/viewer/fixtures/simple-shape.svg"
UNSUPPORTED_FIXTURE = ROOT / "tests/viewer/fixtures/unsupported.svg"
MALFORMED_FIXTURE = ROOT / "tests/viewer/fixtures/malformed.svg"

# Filled from the deterministic P6 outputs produced by frame_compose.c.
EXPECTED_HASHES = {
    "project": "a6fcc40b6ed65966cc9c084348226b028dd36865cb087de6e67c580bfb2583a0",
    "simple": "7988eca2db7ec8b86ca9fbc922eae3df7c95f409e96baec64796a90ff49db79b",
    "unsupported": "27a3530897f51bec6bb21b4f1e083d03557424a92a31ca641ce6b8753efe5529",
}


def read_ppm(path: Path) -> tuple[bytes, bytes]:
    data = path.read_bytes()
    first = data.find(b"\n")
    second = data.find(b"\n", first + 1)
    third = data.find(b"\n", second + 1)
    if first < 0 or second < 0 or third < 0 or data[:first] != b"P6":
        raise AssertionError("output is not a binary P6 PPM")
    return data[: third + 1], data[third + 1 :]


class ViewerFrameTests(unittest.TestCase):
    def setUp(self) -> None:
        (ROOT / ".cache").mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="viewer-frame-", dir=ROOT / ".cache")
        self.output_root = Path(self.temporary.name)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def render(self, fixture: Path, name: str, size: str = "96x72", *extra: str) -> subprocess.CompletedProcess[str]:
        output = self.output_root / f"{name}.ppm"
        return subprocess.run(
            [str(BINARY), "--render-frame", str(fixture), "--size", size,
             "--out", str(output), *extra],
            cwd=ROOT,
            text=True,
            capture_output=True,
            timeout=20,
            check=False,
        )

    def test_project_fixture_hash(self) -> None:
        result = self.render(PROJECT_FIXTURE, "project")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(hashlib.sha256((self.output_root / "project.ppm").read_bytes()).hexdigest(),
                         EXPECTED_HASHES["project"])

    def test_simple_shape_fixture_hash(self) -> None:
        result = self.render(SIMPLE_FIXTURE, "simple", "128x96")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(hashlib.sha256((self.output_root / "simple.ppm").read_bytes()).hexdigest(),
                         EXPECTED_HASHES["simple"])

    def test_unsupported_fixture_writes_explicit_error_frame(self) -> None:
        result = self.render(UNSUPPORTED_FIXTURE, "unsupported", "64x48")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported", result.stderr.lower())
        self.assertIn("byte", result.stderr.lower())
        path = self.output_root / "unsupported.ppm"
        self.assertTrue(path.is_file(), "error frames are written before the command returns failure")
        _, pixels = read_ppm(path)
        self.assertEqual(pixels[:3], bytes((67, 21, 36)))
        self.assertGreater(len(set(pixels[index : index + 3]
                                   for index in range(0, len(pixels), 3))), 1)
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),
                         EXPECTED_HASHES["unsupported"])

    def test_malformed_svg_is_a_bounded_error(self) -> None:
        result = self.render(MALFORMED_FIXTURE, "malformed", "48x32")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("xml", result.stderr.lower())
        self.assertTrue((self.output_root / "malformed.ppm").is_file())

    def test_argument_boundaries_reject_without_traceback(self) -> None:
        invalid = [
            ("8193x1",),
            ("0x1",),
            ("1x8193",),
            ("1x1", "--zoom", "nan"),
            ("1x1", "--zoom", "inf"),
            ("1x1", "--zoom", "64.01"),
            ("1x1", "--pan", "nan,0"),
            ("1x1", "--pan", "0,10000001"),
        ]
        for index, arguments in enumerate(invalid):
            with self.subTest(arguments=arguments):
                size, *extra = arguments
                result = self.render(PROJECT_FIXTURE, f"invalid-{index}", size, *extra)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("render-frame:", result.stderr)
                self.assertNotIn("Traceback", result.stderr)

        too_long_path = "x" * 4097
        result = subprocess.run(
            [str(BINARY), "--render-frame", too_long_path, "--size", "1x1",
             "--out", str(self.output_root / "long-path.ppm")],
            cwd=ROOT,
            text=True,
            capture_output=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_missing_arguments_show_usage_and_exit_two(self) -> None:
        for arguments in ([], [""], ["--unknown"]):
            with self.subTest(arguments=arguments):
                result = subprocess.run([str(BINARY), *arguments], cwd=ROOT, text=True,
                                        capture_output=True, timeout=10, check=False)
                self.assertEqual(result.returncode, 2)
                self.assertIn("usage: svgworkbench", result.stderr)

    def test_zoom_and_pan_render(self) -> None:
        result = self.render(SIMPLE_FIXTURE, "zoom-pan", "128x96",
                             "--zoom", "2.5", "--pan", "8,-4")
        self.assertEqual(result.returncode, 0, result.stderr)
        header, pixels = read_ppm(self.output_root / "zoom-pan.ppm")
        self.assertIn(b"128 96", header)
        self.assertEqual(len(pixels), 128 * 96 * 3)


if __name__ == "__main__":
    unittest.main()
