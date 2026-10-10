import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))


def ppm(width, height, pixel):
    return b"P6\n%d %d\n255\n" % (width, height) + bytes(pixel) * (width * height)


class PixdiffTests(unittest.TestCase):
    def run_diff(self, a, b, *extra):
        with tempfile.TemporaryDirectory() as tmp:
            pa, pb = Path(tmp) / "a.ppm", Path(tmp) / "b.ppm"
            pa.write_bytes(a)
            pb.write_bytes(b)
            return subprocess.run([sys.executable, str(ROOT / "scripts/pixdiff.py"), str(pa), str(pb),
                                   "--out", str(Path(tmp) / "d.png"), *extra],
                                  capture_output=True, text=True)

    def test_identical(self):
        result = self.run_diff(ppm(4, 3, (1, 2, 3)), ppm(4, 3, (1, 2, 3)))
        self.assertEqual(result.returncode, 0)
        self.assertIn("identical", result.stdout)

    def test_reports_count_bbox_and_delta(self):
        a = bytearray(ppm(4, 3, (0, 0, 0)))
        b = bytearray(a)
        header = len(a) - 4 * 3 * 3
        b[header + (1 * 4 + 2) * 3] = 200  # pixel (2,1)
        result = self.run_diff(bytes(a), bytes(b))
        self.assertEqual(result.returncode, 1)
        self.assertIn("1 of 12 pixels differ", result.stdout)
        self.assertIn("max delta 200", result.stdout)
        self.assertIn("bbox x2..2 y1..1", result.stdout)
        self.assertIn("(2,1)", result.stdout)

    def test_tolerance_and_size_mismatch(self):
        near = self.run_diff(ppm(2, 2, (10, 10, 10)), ppm(2, 2, (12, 10, 10)), "--tolerance", "2")
        self.assertEqual(near.returncode, 0)
        self.assertEqual(self.run_diff(ppm(2, 2, (0, 0, 0)), ppm(3, 2, (0, 0, 0))).returncode, 1)


class SelectorTests(unittest.TestCase):
    def test_map_is_valid_and_selects(self):
        import quick_test
        ctest, python, unmapped = quick_test.select(["src/svg/shape/marker.c", "src/ui/ui_core.c", "docs/x.md"])
        joined = "|".join(ctest)
        self.assertIn("svg-marker", joined)
        self.assertIn("ui-", joined)
        self.assertIn("tests.test_shape_render", python)
        self.assertEqual(unmapped, [])
        data = json.loads((ROOT / "tests/test_map.json").read_text())
        for entry in data["map"]:
            self.assertTrue(entry["prefix"] and "ctest" in entry and "python" in entry)

    def test_unmapped_source_is_reported(self):
        import quick_test
        _, _, unmapped = quick_test.select(["newdir/thing.c"])
        self.assertEqual(unmapped, ["newdir/thing.c"])


if __name__ == "__main__":
    unittest.main()
