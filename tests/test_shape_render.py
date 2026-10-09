"""Differential and deterministic integration checks for the bounded shape subset."""
import hashlib
import json
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/svg/shape/fixtures"
CLI = ROOT / "build/mac-debug/bin/shape_render_cli"
OUT = ROOT / "artifacts/WO-M2-05/reference-comparison.json"


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
            if depth != 8 or color != 6 or interlace != 0:
                raise ValueError("reference PNG must be noninterlaced RGBA8")
            bpp = 4
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
    return width, height, b"".join(rows)


class ShapeRenderTests(unittest.TestCase):
    def test_decoder_diagnostics_source_retention_and_raster(self):
        self.assertTrue(CLI.is_file(), "run ./dev build before the focused shape tests")
        result = subprocess.run([str(ROOT / "build/mac-debug/bin/shape_test")], check=True, capture_output=True, text=True)
        self.assertIn("shape decoder and scalar raster checks passed", result.stdout)

    def test_reference_comparison_and_repeatability(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by this work order")
        version = subprocess.run([renderer, "--version"], check=True, capture_output=True, text=True).stdout.strip()
        results = []
        with tempfile.TemporaryDirectory(prefix="vexel-shape-") as temp:
            temp_path = Path(temp)
            for fixture in sorted(FIXTURES.glob("*.svg")):
                raw = fixture.read_bytes()
                first, second, reference = (temp_path / f"{fixture.stem}-{i}.rgba" for i in range(3))
                subprocess.run([str(CLI), str(fixture), str(first)], check=True, capture_output=True)
                subprocess.run([str(CLI), str(fixture), str(second)], check=True, capture_output=True)
                self.assertEqual(first.read_bytes(), second.read_bytes(), fixture.name)
                subprocess.run([renderer, "--format", "png", "--output", str(reference), str(fixture)], check=True, capture_output=True)
                rw, rh, reference_rgba = png_rgba(reference.read_bytes())
                ours = first.read_bytes()
                width, height = 16, 16
                self.assertEqual((rw, rh), (width, height))
                self.assertEqual(len(ours), width * height * 4)
                # Renderer antialiasing may affect pixels within one pixel of a
                # Vexel coverage boundary; every other channel must match exactly.
                bad_interior, edge_diffs, max_channel = 0, 0, 0
                for y in range(height):
                    for x in range(width):
                        offset = (y * width + x) * 4
                        delta = max(abs(ours[offset + c] - reference_rgba[offset + c]) for c in range(4))
                        if not delta:
                            continue
                        max_channel = max(max_channel, delta)
                        edge = False
                        for yy in range(max(0, y - 1), min(height, y + 2)):
                            for xx in range(max(0, x - 1), min(width, x + 2)):
                                oi = (yy * width + xx) * 4 + 3
                                if ours[oi] != ours[offset + 3]:
                                    edge = True
                        if edge:
                            edge_diffs += 1
                        else:
                            bad_interior += 1
                self.assertEqual(bad_interior, 0, fixture.name)
                self.assertLessEqual(edge_diffs, 120, fixture.name)
                results.append({"case": fixture.stem, "source_sha256": hashlib.sha256(raw).hexdigest(),
                                "vexel_sha256": hashlib.sha256(ours).hexdigest(),
                                "reference_png_sha256": hashlib.sha256(reference.read_bytes()).hexdigest(),
                                "edge_diff_pixels": edge_diffs, "interior_diff_pixels": bad_interior,
                                "max_channel_diff": max_channel})
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps({"renderer_path": renderer, "renderer_version": version,
                                   "edge_tolerance": "one pixel around Vexel coverage boundary; interior exact",
                                   "cases": results}, indent=2) + "\n")

    def test_bounds_and_stable_diagnostics(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        cases = {
            "huge": '<svg width="16" height="16"><path d="M0 0 L1000001 1"/></svg>',
            "nonfinite": '<svg width="16" height="16"><path d="M0 0 Lnan 1"/></svg>',
            "malformed": '<svg width="16" height="16"><path d="M,,0 1"/></svg>',
            "invalid_arc_flag": '<svg width="16" height="16"><path d="M0 0 A1 1 0 2 0 4 4"/></svg>',
            "nonliteral_arc_flag": '<svg width="16" height="16"><path d="M0 0 A1 1 0 1.0 0 4 4"/></svg>',
            "missing_initial_move": '<svg width="16" height="16"><path d="L0 0"/></svg>',
            "unsupported_smooth": '<svg width="16" height="16"><path d="M0 0 S1 1 2 2"/></svg>',
        }
        report = {"cases": [], "source_retention": "input bytes hashed before and after invocation; CLI borrows file unchanged"}
        with tempfile.TemporaryDirectory(prefix="vexel-shape-bounds-") as temp:
            temp_path = Path(temp)
            for name, source in cases.items():
                src, out = temp_path / f"{name}.svg", temp_path / f"{name}.rgba"
                src.write_text(source)
                digest_before = hashlib.sha256(src.read_bytes()).hexdigest()
                run = subprocess.run([str(binary), str(src), str(out)], capture_output=True, text=True)
                digest_after = hashlib.sha256(src.read_bytes()).hexdigest()
                self.assertNotEqual(run.returncode, 0, name)
                category, offset = run.stderr.strip().split(":", 1)
                self.assertTrue(offset.isdigit(), name)
                self.assertEqual(digest_before, digest_after, name)
                report["cases"].append({"name": name, "category": category, "offset": int(offset),
                                        "source_sha256": digest_before, "source_unchanged": True})
            # Curved groups exhaust the edge cap before the 65,536 command cap.
            path = "M0 0 " + " ".join("C1 15 2 -10 3 3" for _ in range(65535))
            src = temp_path / "edge-cap.svg"
            src.write_text(f'<svg width="16" height="16"><path d="{path}"/></svg>')
            run = subprocess.run([str(binary), str(src), str(temp_path / "edge-cap.rgba")], capture_output=True, text=True)
            self.assertNotEqual(run.returncode, 0)
            category, offset = run.stderr.strip().split(":", 1)
            self.assertEqual(category, "limit")
            report["cases"].append({"name": "flattened_edge_cap", "category": category, "offset": int(offset),
                                    "generated_curve_groups": 65535})
            degenerate = temp_path / "degenerate.svg"
            degenerate.write_text('<svg width="16" height="16"><path d="M2 2 L2 2 Z"/></svg>')
            run = subprocess.run([str(binary), str(degenerate), str(temp_path / "degenerate.rgba")], capture_output=True)
            self.assertEqual(run.returncode, 0)
            report["cases"].append({"name": "degenerate_path", "result": "safe_empty_or_filled"})
        target = ROOT / "artifacts/WO-M2-05/bounds-and-diagnostics.json"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(report, indent=2) + "\n")

    def test_independent_shape_fill_rules_and_painter_order(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = (b'<svg width="16" height="16">'
                  b'<rect x="1" y="1" width="11" height="11" fill="green"/>'
                  b'<rect x="5" y="5" width="10" height="10" fill="red" fill-rule="evenodd"/>'
                  b'<path fill="blue" fill-rule="evenodd" '
                  b'd="M2 2 H14 V14 H2 M5 5 H11 V11 H5"/>'
                  b'</svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-shape-order-") as temp:
            input_path, output_path = Path(temp) / "order.svg", Path(temp) / "order.rgba"
            input_path.write_bytes(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            rgba = output_path.read_bytes()
        pixel = lambda x, y: rgba[(y * 16 + x) * 4:(y * 16 + x) * 4 + 4]
        self.assertEqual(pixel(1, 1), bytes((0, 128, 0, 255)))
        self.assertEqual(pixel(7, 7), bytes((255, 0, 0, 255)))
        self.assertEqual(pixel(12, 12), bytes((0, 0, 255, 255)))
        self.assertEqual(pixel(8, 8), bytes((255, 0, 0, 255)))

    def test_fractional_viewport_rounds_tile_extent_up(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = b'<svg width="8.5px" height="7.25"/>'
        with tempfile.TemporaryDirectory(prefix="vexel-shape-fractional-") as temp:
            input_path, output_path = Path(temp) / "fractional.svg", Path(temp) / "fractional.rgba"
            input_path.write_bytes(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            self.assertEqual(output_path.stat().st_size, 9 * 8 * 4)


if __name__ == "__main__":
    unittest.main()
