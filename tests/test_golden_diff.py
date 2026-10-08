"""Tests for deterministic, hash-pinned PNG golden comparison."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import golden_diff


def metadata(fixture: dict[str, object], actual: Path, report: Path, heatmap: Path, **kw: object) -> Namespace:
    return Namespace(fixture=fixture["fixture_id"], actual=str(actual), report=str(report), heatmap=str(heatmap),
                     svg_sha256=fixture["source_sha256"], build_sha="a" * 64, scale=1.0, frame=0,
                     font="test-font", backend="test-backend", edge_radius=kw.get("edge_radius", 1),
                     edge_channel_tolerance=kw.get("edge_channel_tolerance", 1))


class GoldenDiffTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        lock = json.loads((ROOT / "tests/golden/manifest.lock").read_text(encoding="utf-8"))
        cls.fixture = next(item for item in lock["fixtures"] if item["fixture_id"] == "shapes-basic")
        cls.baseline = ROOT / cls.fixture["png_path"]

    def test_identical_image_has_exact_zero_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            args = metadata(self.fixture, self.baseline, root / "report", root / "heatmap")
            report, heat = golden_diff.compare(args)
            self.assertTrue(report["metrics"]["exact_equal"])
            self.assertEqual(report["metrics"]["differing_pixels"], 0)
            self.assertEqual(report["tolerance"], {"tolerated_pixels": 0, "non_tolerated_pixels": 0})
            self.assertEqual(golden_diff.decode_png(heat)[2], bytes(256 * 256 * 4))

    def test_small_channel_difference_on_edge_is_tolerated(self) -> None:
        width = height = 3
        expected = bytearray([0, 0, 0, 255] * 9)
        expected[(1 * width + 1) * 4 : (1 * width + 2) * 4] = bytes([255, 255, 255, 255])
        actual = bytearray(expected)
        actual[(1 * width + 0) * 4] = 1
        expected_png = golden_diff.encode_png(width, height, bytes(expected))
        actual_png = golden_diff.encode_png(width, height, bytes(actual))
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "baseline.png").write_bytes(expected_png)
            (root / "actual.png").write_bytes(actual_png)
            fixture = {**self.fixture, "png_path": str(root / "baseline.png"),
                       "png_sha256": hashlib.sha256(expected_png).hexdigest(), "width": width, "height": height}
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"fixtures": [fixture]}), encoding="utf-8")
            args = metadata(fixture, root / "actual.png", root / "report", root / "heatmap")
            with patch.object(golden_diff, "MANIFEST", manifest):
                report, heat = golden_diff.compare(args)
            self.assertEqual(report["tolerance"], {"tolerated_pixels": 1, "non_tolerated_pixels": 0})
            self.assertEqual(report["metrics"]["differing_channels"], 1)
            self.assertEqual(golden_diff.decode_png(heat)[2][width * 4 : width * 4 + 4],
                             bytes([255, 255, 0, 255]))
            args.edge_radius = 0
            with patch.object(golden_diff, "MANIFEST", manifest):
                zero_radius_report, _ = golden_diff.compare(args)
            self.assertEqual(zero_radius_report["tolerance"],
                             {"tolerated_pixels": 1, "non_tolerated_pixels": 0})

    def test_shift_with_zero_tolerance_is_not_tolerated(self) -> None:
        width = height = 3
        expected = bytearray([0, 0, 0, 255] * 9)
        expected[(1 * width + 1) * 4 : (1 * width + 2) * 4] = bytes([255, 255, 255, 255])
        actual = bytearray([0, 0, 0, 255] * 9)
        actual[(1 * width + 2) * 4 : (1 * width + 3) * 4] = bytes([255, 255, 255, 255])
        expected_png = golden_diff.encode_png(width, height, bytes(expected))
        actual_png = golden_diff.encode_png(width, height, bytes(actual))
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "baseline.png").write_bytes(expected_png)
            (root / "actual.png").write_bytes(actual_png)
            fixture = {**self.fixture, "png_path": str(root / "baseline.png"),
                       "png_sha256": hashlib.sha256(expected_png).hexdigest(), "width": width, "height": height}
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"fixtures": [fixture]}), encoding="utf-8")
            args = metadata(fixture, root / "actual.png", root / "report", root / "heatmap",
                            edge_radius=0, edge_channel_tolerance=0)
            with patch.object(golden_diff, "MANIFEST", manifest):
                report, _ = golden_diff.compare(args)
            self.assertEqual(report["tolerance"], {"tolerated_pixels": 0, "non_tolerated_pixels": 2})

    def test_missing_baseline_is_an_error(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            fixture = {**self.fixture, "png_path": str(root / "missing.png")}
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"fixtures": [fixture]}), encoding="utf-8")
            args = metadata(fixture, self.baseline, root / "report", root / "heatmap")
            with patch.object(golden_diff, "MANIFEST", manifest):
                with self.assertRaisesRegex(golden_diff.CompareError, "cannot read PNG"):
                    golden_diff.compare(args)

    def test_bad_crc_and_filter_are_rejected(self) -> None:
        valid = self.baseline.read_bytes()
        corrupt_crc = bytearray(valid)
        corrupt_crc[29] ^= 1
        with self.assertRaisesRegex(golden_diff.CompareError, "bad CRC"):
            golden_diff.decode_png(bytes(corrupt_crc))
        width = height = 1
        png = bytearray(golden_diff.encode_png(width, height, bytes([0, 0, 0, 255])))
        import struct
        import zlib
        # Replace the first scanline's filter with an unsupported filter and refresh IDAT CRC.
        idat = png.index(b"IDAT")
        compressed = zlib.decompress(bytes(png[idat + 4 : idat + 4 + struct.unpack_from(">I", png, idat - 4)[0]]))
        malformed = zlib.compress(bytes([5]) + compressed[1:])
        start = idat - 4
        old_length = struct.unpack_from(">I", png, start)[0]
        end = start + 12 + old_length
        replacement = struct.pack(">I", len(malformed)) + b"IDAT" + malformed
        replacement += struct.pack(">I", zlib.crc32(b"IDAT" + malformed) & 0xFFFFFFFF)
        png[start:end] = replacement
        with self.assertRaisesRegex(golden_diff.CompareError, "unsupported PNG filter"):
            golden_diff.decode_png(bytes(png))
        valid = golden_diff.encode_png(1, 1, bytes([0, 0, 0, 255]))
        ihdr_end = 8 + 25
        idat_end = ihdr_end + 12 + struct.unpack_from(">I", valid, ihdr_end)[0]
        bkgd_data = b"\x00\x00\x00\x00\x00\x00"
        bkgd = struct.pack(">I", len(bkgd_data)) + b"bKGD" + bkgd_data
        bkgd += struct.pack(">I", zlib.crc32(b"bKGD" + bkgd_data) & 0xFFFFFFFF)
        malformed_order = valid[:idat_end] + bkgd + valid[idat_end:]
        with self.assertRaisesRegex(golden_diff.CompareError, "bKGD chunk must precede IDAT"):
            golden_diff.decode_png(malformed_order)

    def test_svg_metadata_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            args = metadata(self.fixture, self.baseline, root / "report", root / "heatmap")
            args.svg_sha256 = "b" * 64
            with self.assertRaisesRegex(golden_diff.CompareError, "does not match"):
                golden_diff.compare(args)

    def test_report_and_heatmap_are_byte_deterministic(self) -> None:
        data = self.baseline.read_bytes()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            args = metadata(self.fixture, self.baseline, root / "report", root / "heatmap")
            report, heatmap = golden_diff.compare(args)
            first_report = (json.dumps(report, sort_keys=True, indent=2) + "\n").encode()
            report2, heatmap2 = golden_diff.compare(args)
            second_report = (json.dumps(report2, sort_keys=True, indent=2) + "\n").encode()
            self.assertEqual(first_report, second_report)
            self.assertEqual(heatmap, heatmap2)
            self.assertEqual(data, self.baseline.read_bytes())

    def test_output_path_escape_and_symlink_are_rejected(self) -> None:
        with self.assertRaisesRegex(golden_diff.CompareError, "under repository .cache"):
            golden_diff._output_path("/tmp/out.json")
        with self.assertRaisesRegex(golden_diff.CompareError, "under repository .cache"):
            golden_diff._output_path(".cache/../work/escaped.json")
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / ".cache").mkdir()
            (root / ".cache/link").symlink_to(root, target_is_directory=True)
            with self.assertRaisesRegex(golden_diff.CompareError, "symlinks"):
                golden_diff._output_path(str(root / ".cache/link/out.json"), root)

    def test_existing_output_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / ".cache").mkdir()
            output = root / ".cache/report.json"
            output.write_text("retained", encoding="utf-8")
            with self.assertRaisesRegex(golden_diff.CompareError, "overwrite"):
                golden_diff._output_path(str(output), root)
            self.assertEqual(output.read_text(encoding="utf-8"), "retained")


if __name__ == "__main__":
    unittest.main()
